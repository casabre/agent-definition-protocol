package adp

import (
	"fmt"
	"net/url"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"gopkg.in/yaml.v3"
)

const maxDepth = 10

// Resolver is a function that loads a URI and returns YAML bytes.
type Resolver func(uri string) ([]byte, error)

// CompositionError is returned for any composition-time failure.
type CompositionError struct {
	msg string
}

func (e *CompositionError) Error() string { return e.msg }

func compositionError(format string, args ...interface{}) *CompositionError {
	return &CompositionError{msg: fmt.Sprintf(format, args...)}
}

// ResolveADP loads and fully resolves an ADP manifest, applying extends/import/overrides.
// It returns the resolved *ADP and a (possibly empty) slice of validation error strings.
// If the manifest cannot be loaded or composed, it returns (nil, errors).
func ResolveADP(path string, resolver Resolver) (*ADP, []string) {
	absPath, _ := filepath.Abs(path) // filepath.Abs cannot fail on any supported platform
	raw, err := loadURI(absPath, resolver)
	if err != nil {
		return nil, []string{err.Error()}
	}

	merged, compErr := resolveManifest(raw, absPath, make(map[string]bool), 0, resolver)
	if compErr != nil {
		return nil, []string{compErr.Error()}
	}

	// Marshal back to YAML then unmarshal into typed ADP struct.
	// yaml.Marshal on map[string]interface{} cannot fail; yaml.Unmarshal on
	// the resulting bytes cannot fail either.
	yamlBytes, _ := yaml.Marshal(merged)
	var adp ADP
	_ = yaml.Unmarshal(yamlBytes, &adp)

	// Semantic validation.
	semErrors := ValidateADPSemantics(&adp)
	if len(semErrors) > 0 {
		return nil, semErrors
	}
	return &adp, nil
}

// resolveManifest recursively resolves a raw manifest map.
func resolveManifest(
	data map[string]interface{},
	baseURI string,
	seen map[string]bool,
	depth int,
	resolver Resolver,
) (map[string]interface{}, *CompositionError) {
	if depth > maxDepth {
		return nil, compositionError("extends chain depth exceeded %d", maxDepth)
	}
	if seen[baseURI] {
		return nil, compositionError("circular extends detected: %s", baseURI)
	}
	// Clone seen set so sibling branches don't interfere.
	newSeen := make(map[string]bool, len(seen)+1)
	for k, v := range seen {
		newSeen[k] = v
	}
	newSeen[baseURI] = true

	merged := map[string]interface{}{}

	// Step 1: Apply extended base if present.
	if extendsRaw, ok := data["extends"]; ok {
		extendsURI, _ := extendsRaw.(string)
		if extendsURI != "" {
			absURI, err := resolveURI(extendsURI, baseURI)
			if err != nil {
				return nil, err
			}
			baseRaw, loadErr := loadURI(absURI, resolver)
			if loadErr != nil {
				return nil, compositionError("%v", loadErr)
			}
			baseResolved, compErr := resolveManifest(baseRaw, absURI, newSeen, depth+1, resolver)
			if compErr != nil {
				return nil, compErr
			}
			merged = deepMerge(merged, baseResolved)
		}
	}

	// Step 2: Apply local fields using id-keyed merge semantics:
	// objects deep-merge; id-carrying lists merge by id; other lists replace.
	local := map[string]interface{}{}
	for k, v := range data {
		if k == "extends" || k == "import" || k == "overrides" {
			continue
		}
		local[k] = v
	}
	merged = applyPatch(merged, local)

	// Step 3: Additively merge import entries.
	if importsRaw, ok := data["import"]; ok {
		imports, _ := importsRaw.([]interface{})
		for _, entryRaw := range imports {
			entry, _ := entryRaw.(map[string]interface{})
			if entry == nil {
				continue
			}
			fromURI, _ := entry["from"].(string)
			if fromURI == "" {
				return nil, compositionError("import entry missing 'from' field")
			}
			absURI, err := resolveURI(fromURI, baseURI)
			if err != nil {
				return nil, err
			}
			moduleRaw, loadErr := loadURI(absURI, resolver)
			if loadErr != nil {
				return nil, compositionError("%v", loadErr)
			}
			// Filter to requested sections if specified.
			if sectionsRaw, ok := entry["sections"]; ok {
				sections := toStringSlice(sectionsRaw)
				if len(sections) > 0 {
					filtered := map[string]interface{}{}
					for _, sec := range sections {
						if v, ok := moduleRaw[sec]; ok {
							filtered[sec] = v
						}
					}
					moduleRaw = filtered
				}
			}
			merged = additiveMerge(merged, moduleRaw)
		}
	}

	// Step 4: Apply overrides.
	if overridesRaw, ok := data["overrides"]; ok {
		overrides, _ := overridesRaw.([]interface{})
		for _, ovRaw := range overrides {
			ov, _ := ovRaw.(map[string]interface{})
			if ov == nil {
				continue
			}
			var compErr *CompositionError
			merged, compErr = applyOverride(merged, ov)
			if compErr != nil {
				return nil, compErr
			}
		}
	}

	return merged, nil
}

// deepMerge implements RFC 7396 JSON Merge Patch: overlay wins; null removes; arrays replace.
func deepMerge(base, overlay map[string]interface{}) map[string]interface{} {
	result := shallowCopy(base)
	for k, v := range overlay {
		if v == nil {
			delete(result, k)
		} else if overlayMap, ok := v.(map[string]interface{}); ok {
			if baseMap, ok := result[k].(map[string]interface{}); ok {
				result[k] = deepMerge(baseMap, overlayMap)
			} else {
				result[k] = deepCopyValue(v)
			}
		} else {
			result[k] = deepCopyValue(v)
		}
	}
	return result
}

// additiveMerge merges module into base: arrays append; objects deep-merge; scalars: module wins.
func additiveMerge(base, module map[string]interface{}) map[string]interface{} {
	result := shallowCopy(base)
	for k, v := range module {
		if _, exists := result[k]; !exists {
			result[k] = deepCopyValue(v)
		} else if modList, ok := v.([]interface{}); ok {
			if baseList, ok := result[k].([]interface{}); ok {
				combined := make([]interface{}, len(baseList)+len(modList))
				copy(combined, baseList)
				copy(combined[len(baseList):], modList)
				result[k] = combined
			} else {
				result[k] = deepCopyValue(v)
			}
		} else if modMap, ok := v.(map[string]interface{}); ok {
			if baseMap, ok := result[k].(map[string]interface{}); ok {
				result[k] = additiveMerge(baseMap, modMap)
			} else {
				result[k] = deepCopyValue(v)
			}
		} else {
			result[k] = deepCopyValue(v)
		}
	}
	return result
}

// applyPatch applies a structural patch: objects deep-merge; id-keyed lists merge by id; other lists replace.
func applyPatch(base, patch map[string]interface{}) map[string]interface{} {
	result := shallowCopy(base)
	for k, v := range patch {
		if v == nil {
			delete(result, k)
		} else if patchMap, ok := v.(map[string]interface{}); ok {
			if baseMap, ok := result[k].(map[string]interface{}); ok {
				result[k] = applyPatch(baseMap, patchMap)
			} else {
				result[k] = deepCopyValue(v)
			}
		} else if patchList, ok := v.([]interface{}); ok {
			if baseList, ok := result[k].([]interface{}); ok {
				if allHaveID(patchList) {
					result[k] = idKeyedMerge(baseList, patchList)
				} else {
					result[k] = deepCopyValue(v)
				}
			} else {
				result[k] = deepCopyValue(v)
			}
		} else {
			result[k] = deepCopyValue(v)
		}
	}
	return result
}

// idKeyedMerge merges two lists by "id" field. Matched entries are deep-patched;
// unknown patch ids are appended; unmatched base entries are kept.
func idKeyedMerge(baseList, patchList []interface{}) []interface{} {
	result := make([]interface{}, len(baseList))
	for i, item := range baseList {
		result[i] = deepCopyValue(item)
	}
	index := map[string]int{}
	for i, item := range result {
		if m, ok := item.(map[string]interface{}); ok {
			if id, ok := m["id"].(string); ok {
				index[id] = i
			}
		}
	}
	for _, patchItem := range patchList {
		patchMap, ok := patchItem.(map[string]interface{})
		if !ok {
			result = append(result, deepCopyValue(patchItem))
			continue
		}
		id, hasID := patchMap["id"].(string)
		if !hasID {
			result = append(result, deepCopyValue(patchItem))
			continue
		}
		if idx, exists := index[id]; exists {
			if baseEntry, ok := result[idx].(map[string]interface{}); ok {
				result[idx] = applyPatch(baseEntry, patchMap)
			}
		} else {
			result = append(result, deepCopyValue(patchItem))
		}
	}
	return result
}

// allHaveID returns true iff the list is non-empty and every element is a map with an "id" key.
func allHaveID(list []interface{}) bool {
	if len(list) == 0 {
		return false
	}
	for _, item := range list {
		m, ok := item.(map[string]interface{})
		if !ok {
			return false
		}
		if _, hasID := m["id"]; !hasID {
			return false
		}
	}
	return true
}

// applyOverride applies a single override entry using JSON Pointer–style paths.
func applyOverride(data map[string]interface{}, override map[string]interface{}) (map[string]interface{}, *CompositionError) {
	pathStr, _ := override["path"].(string)
	if !strings.HasPrefix(pathStr, "/") {
		return nil, compositionError("override path must start with '/': %q", pathStr)
	}
	op := "set"
	if opRaw, ok := override["op"]; ok {
		op, _ = opRaw.(string)
	}
	value := override["value"]

	rawSegments := strings.Split(strings.TrimPrefix(pathStr, "/"), "/")
	segments := make([]string, len(rawSegments))
	for i, s := range rawSegments {
		segments[i] = unescapePointer(s)
	}

	result := deepCopyMap(data)

	if op == "delete" {
		node := interface{}(result)
		for _, seg := range segments[:len(segments)-1] {
			next, err := pointerGet(node, seg, pathStr, true)
			if err != nil {
				return nil, err
			}
			if next == nil {
				return result, nil
			}
			node = next
		}
		last := segments[len(segments)-1]
		if nodeMap, ok := node.(map[string]interface{}); ok {
			delete(nodeMap, last)
		}
		return result, nil
	}

	// For set / append: navigate to parent.
	node := interface{}(result)
	for _, seg := range segments[:len(segments)-1] {
		next, err := pointerGet(node, seg, pathStr, false)
		if err != nil {
			return nil, err
		}
		node = next
	}
	last := segments[len(segments)-1]

	switch op {
	case "set":
		if nodeMap, ok := node.(map[string]interface{}); ok {
			if _, exists := nodeMap[last]; !exists {
				return nil, compositionError("override set: path %q does not exist", pathStr)
			}
			nodeMap[last] = deepCopyValue(value)
		} else if nodeList, ok := node.([]interface{}); ok {
			idx, err := toIndex(last, pathStr)
			if err != nil {
				return nil, err
			}
			nodeList[idx] = deepCopyValue(value)
		} else {
			return nil, compositionError("override set: cannot navigate path %q", pathStr)
		}
	case "append":
		target, err := pointerGet(node, last, pathStr, false)
		if err != nil {
			return nil, err
		}
		targetList, ok := target.([]interface{})
		if !ok {
			return nil, compositionError("override append: path %q does not resolve to an array", pathStr)
		}
		targetList = append(targetList, deepCopyValue(value))
		// Write back — the list is a copy so we need to set it on the parent.
		if nodeMap, ok := node.(map[string]interface{}); ok {
			nodeMap[last] = targetList
		}
	default:
		return nil, compositionError("unknown override op: %q", op)
	}

	return result, nil
}

// pointerGet traverses a single path segment on node.
func pointerGet(node interface{}, segment, path string, allowMissing bool) (interface{}, *CompositionError) {
	switch n := node.(type) {
	case map[string]interface{}:
		if allowMissing {
			return n[segment], nil
		}
		v, ok := n[segment]
		if !ok {
			return nil, compositionError("override: path segment %q not found (path: %q)", segment, path)
		}
		return v, nil
	case []interface{}:
		idx, err := toIndex(segment, path)
		if err != nil {
			return nil, err
		}
		return n[idx], nil
	default:
		return nil, compositionError("override: cannot navigate into %T at path %q", node, path)
	}
}

func toIndex(segment, path string) (int, *CompositionError) {
	idx, err := strconv.Atoi(segment)
	if err != nil {
		return 0, compositionError("override: array index %q is not an integer (path: %q)", segment, path)
	}
	return idx, nil
}

func unescapePointer(segment string) string {
	s := strings.ReplaceAll(segment, "~1", "/")
	return strings.ReplaceAll(s, "~0", "~")
}

// resolveURI resolves a (possibly relative) URI against a base URI.
func resolveURI(uri, baseURI string) (string, *CompositionError) {
	parsed, err := url.Parse(uri)
	if err != nil {
		return "", compositionError("invalid URI %q: %v", uri, err)
	}
	if parsed.Scheme == "http" || parsed.Scheme == "https" || parsed.Scheme == "file" {
		return uri, nil
	}
	if parsed.Scheme == "registry" {
		return "", compositionError("registry:// URIs are not supported in v0.2.0; planned for v0.3.0: %q", uri)
	}
	// Relative path: resolve against base file path.
	// filepath.Abs cannot fail for a valid joined path.
	base := filepath.Dir(baseURI)
	abs, _ := filepath.Abs(filepath.Join(base, uri))
	return abs, nil
}

// loadURI loads YAML from a URI (file or via custom resolver).
func loadURI(uri string, resolver Resolver) (map[string]interface{}, error) {
	var data []byte
	var err error
	if resolver != nil {
		data, err = resolver(uri)
		if err != nil {
			return nil, fmt.Errorf("resolver failed for %q: %w", uri, err)
		}
	} else {
		parsed, _ := url.Parse(uri)
		if parsed != nil && (parsed.Scheme == "http" || parsed.Scheme == "https") {
			return nil, fmt.Errorf("HTTP URIs require a custom Resolver: %q", uri)
		}
		// Treat as file path.
		filePath := uri
		if parsed != nil && parsed.Scheme == "file" {
			filePath = parsed.Path
		}
		data, err = os.ReadFile(filePath)
		if err != nil {
			return nil, fmt.Errorf("cannot resolve URI %q: %w", uri, err)
		}
	}
	var raw map[string]interface{}
	if err := yaml.Unmarshal(data, &raw); err != nil {
		return nil, fmt.Errorf("YAML parse error for %q: %w", uri, err)
	}
	if raw == nil {
		raw = map[string]interface{}{}
	}
	return raw, nil
}

// ---- helpers ----------------------------------------------------------------

func shallowCopy(m map[string]interface{}) map[string]interface{} {
	out := make(map[string]interface{}, len(m))
	for k, v := range m {
		out[k] = v
	}
	return out
}

func deepCopyMap(m map[string]interface{}) map[string]interface{} {
	out := make(map[string]interface{}, len(m))
	for k, v := range m {
		out[k] = deepCopyValue(v)
	}
	return out
}

func deepCopyValue(v interface{}) interface{} {
	switch val := v.(type) {
	case map[string]interface{}:
		return deepCopyMap(val)
	case []interface{}:
		out := make([]interface{}, len(val))
		for i, item := range val {
			out[i] = deepCopyValue(item)
		}
		return out
	default:
		return v
	}
}

func toStringSlice(v interface{}) []string {
	raw, ok := v.([]interface{})
	if !ok {
		return nil
	}
	out := make([]string, 0, len(raw))
	for _, item := range raw {
		if s, ok := item.(string); ok {
			out = append(out, s)
		}
	}
	return out
}
