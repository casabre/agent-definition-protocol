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

type Runtime struct {
    Execution []RuntimeEntry `yaml:"execution"`
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
