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

func (client *Client) GetTeams() (*TeamsResponse, error) {
	query := `
		query Teams {
			teams {
				nodes {
					id
					name
          issueCount
				}
			}
		}
	`

	var response TeamsResponse
	if err := client.makeRequest(query, nil, &response); err != nil {
		return nil, err
	}

	return &response, nil
}

func (client *Client) GetTeamIssues(teamId string) (*TeamIssuesResponse, error) {
	query := `
    query TeamIssues($teamId: String!) {
      team(id: $teamId) {
        issues {
          nodes {
            id
            title
            description
          }
        }
      }
    }
  `

	variables := map[string]interface{}{
		"teamId": teamId,
	}

	var response TeamIssuesResponse
	if err := client.makeRequest(query, variables, &response); err != nil {
		return nil, err
	}

	return &response, nil
}

func (client *Client) makeRequest(query interface{}, variables map[string]interface{}, response interface{}) error {
	requestBody := map[string]interface{}{
		"query":     query,
		"variables": variables,
	}

	jsonBytes, err := json.Marshal(requestBody)
	if err != nil {
		return fmt.Errorf("marshal request: %w", err)
	}

	httpReq, err := http.NewRequest("POST", apiEndpoint, bytes.NewBuffer(jsonBytes))
	if err != nil {
		return fmt.Errorf("create request: %w", err)
	}

	httpReq.Header.Set("Content-Type", "application/json")
	httpReq.Header.Set("Authorization", client.apiKey)

	httpResp, err := client.httpClient.Do(httpReq)
	if err != nil {
		return fmt.Errorf("do request: %w", err)
	}
	defer httpResp.Body.Close()

	body, err := io.ReadAll(httpResp.Body)
	if err != nil {
		return fmt.Errorf("read response: %w", err)
	}

	if err := json.Unmarshal(body, response); err != nil {
		return fmt.Errorf("unmarshal response: %w", err)
	}

	return nil
}
