package api

type Team struct {
	ID         string `json:"id"`
	Name       string `json:"name"`
	IssueCount int    `json:"issueCount"`
}

type Issue struct {
	ID          string  `json:"id"`
	Title       string  `json:"title"`
	Description string  `json:"description"`
	SortOrder   float32 `json:"sortOrder"`
}

type Teams struct {
	Nodes []Team `json:"nodes"`
}

type TeamsResponse struct {
	Data struct {
		Teams Teams `json:"teams"`
	} `json:"data"`
}

type TeamIssuesResponse struct {
	Data struct {
		Team struct {
			Issues struct {
				Nodes []Issue `json:"nodes"`
			} `json:"issues"`
		} `json:"team"`
	} `son:"data"`
}
