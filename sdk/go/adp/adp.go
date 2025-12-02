package adp

import (
    "io/ioutil"
    "gopkg.in/yaml.v3"
)

type RuntimeEntry struct {
    Backend string `yaml:"backend"`
    ID      string `yaml:"id"`
    Entrypoint string `yaml:"entrypoint"`
}

type Model struct {
    ID          string                 `yaml:"id"`
    Provider    string                 `yaml:"provider"`
    Model       string                 `yaml:"model"`
    APIKeyEnv   string                 `yaml:"api_key_env,omitempty"`
    BaseURL     string                 `yaml:"base_url,omitempty"`
    Temperature *float64               `yaml:"temperature,omitempty"`
    MaxTokens   *int                   `yaml:"max_tokens,omitempty"`
    Extensions  map[string]interface{} `yaml:"extensions,omitempty"`
}

type Runtime struct {
    Execution []RuntimeEntry `yaml:"execution"`
    Models   []Model        `yaml:"models,omitempty"`
}

type ADP struct {
    ADPVersion string      `yaml:"adp_version"`
    ID         string      `yaml:"id"`
    Runtime    Runtime     `yaml:"runtime"`
    Flow       interface{} `yaml:"flow"`
    Evaluation interface{} `yaml:"evaluation"`
}

func LoadADP(path string) (*ADP, error) {
    data, err := ioutil.ReadFile(path)
    if err != nil {
        return nil, err
    }
    var adp ADP
    if err := yaml.Unmarshal(data, &adp); err != nil {
        return nil, err
    }
    return &adp, nil
}
