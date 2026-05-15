package adp

import (
    "os"
    "gopkg.in/yaml.v3"
)

type RuntimeEntry struct {
    Backend        string `yaml:"backend"          json:"backend"`
    ID             string `yaml:"id"               json:"id"`
    Entrypoint     string `yaml:"entrypoint,omitempty"  json:"entrypoint,omitempty"`
    Image          string `yaml:"image,omitempty"       json:"image,omitempty"`
    Module         string `yaml:"module,omitempty"      json:"module,omitempty"`
    Path           string `yaml:"path,omitempty"        json:"path,omitempty"`
    BackendType    string `yaml:"type,omitempty"        json:"type,omitempty"`
    Endpoint       string `yaml:"endpoint,omitempty"    json:"endpoint,omitempty"`
    PackageManager string `yaml:"package_manager,omitempty" json:"package_manager,omitempty"`
}

type Model struct {
    ID          string                 `yaml:"id"           json:"id"`
    Provider    string                 `yaml:"provider"     json:"provider"`
    Model       string                 `yaml:"model"        json:"model"`
    APIKeyEnv   string                 `yaml:"api_key_env,omitempty"  json:"api_key_env,omitempty"`
    BaseURL     string                 `yaml:"base_url,omitempty"     json:"base_url,omitempty"`
    Temperature *float64               `yaml:"temperature,omitempty"  json:"temperature,omitempty"`
    MaxTokens   *int                   `yaml:"max_tokens,omitempty"   json:"max_tokens,omitempty"`
    Extensions  map[string]interface{} `yaml:"extensions,omitempty"   json:"extensions,omitempty"`
}

type Runtime struct {
    Execution []RuntimeEntry `yaml:"execution"       json:"execution"`
    Models    []Model        `yaml:"models,omitempty" json:"models,omitempty"`
}

type ADP struct {
    ADPVersion string      `yaml:"adp_version"        json:"adp_version"`
    ID         string      `yaml:"id"                 json:"id"`
    Name       string      `yaml:"name,omitempty"     json:"name,omitempty"`
    Description string     `yaml:"description,omitempty" json:"description,omitempty"`
    Owner      string      `yaml:"owner,omitempty"    json:"owner,omitempty"`
    Tags       []string    `yaml:"tags,omitempty"     json:"tags,omitempty"`
    Runtime    Runtime     `yaml:"runtime"            json:"runtime"`
    Flow       interface{} `yaml:"flow"               json:"flow"`
    Evaluation interface{} `yaml:"evaluation"         json:"evaluation"`
}

func LoadADP(path string) (*ADP, error) {
    data, err := os.ReadFile(path)
    if err != nil {
        return nil, err
    }
    var adp ADP
    if err := yaml.Unmarshal(data, &adp); err != nil {
        return nil, err
    }
    return &adp, nil
}
