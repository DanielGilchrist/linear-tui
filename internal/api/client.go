package api

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
)

const apiEndpoint = "https://api.linear.app/graphql"

type Client struct {
	httpClient *http.Client
	apiKey     string
}

func NewClient(apiKey string) *Client {
	return &Client{
		httpClient: &http.Client{},
		apiKey:     apiKey,
	}
}

func (c *Client) GetTeamsWithIssues() (*TeamsResponse, error) {
	query := `
		query TeamsWithIssues {
			teams {
				nodes {
					id
					name
					issues {
						nodes {
							id
							title
							description
							sortOrder
						}
					}
					issueCount
				}
			}
		}
	`

	req := struct {
		Query string `json:"query"`
	}{Query: query}

	var resp TeamsResponse
	if err := c.makeRequest(req, &resp); err != nil {
		return nil, err
	}

	return &resp, nil
}

func (c *Client) makeRequest(req interface{}, resp interface{}) error {
	jsonBytes, err := json.Marshal(req)
	if err != nil {
		return fmt.Errorf("marshal request: %w", err)
	}

	httpReq, err := http.NewRequest("POST", apiEndpoint, bytes.NewBuffer(jsonBytes))
	if err != nil {
		return fmt.Errorf("create request: %w", err)
	}

	httpReq.Header.Set("Content-Type", "application/json")
	httpReq.Header.Set("Authorization", c.apiKey)

	httpResp, err := c.httpClient.Do(httpReq)
	if err != nil {
		return fmt.Errorf("do request: %w", err)
	}
	defer httpResp.Body.Close()

	body, err := io.ReadAll(httpResp.Body)
	if err != nil {
		return fmt.Errorf("read response: %w", err)
	}

	if err := json.Unmarshal(body, resp); err != nil {
		return fmt.Errorf("unmarshal response: %w", err)
	}

	return nil
}
