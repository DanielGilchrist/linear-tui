package api

type Comment struct {
	Body string `json:"body"`
}

type Issue struct {
	Title       string `json:"title"`
	Description string `json:"description"`
	Comments    struct {
		Nodes []Comment `json:"nodes"`
	} `json:"comments"`
	URL string `json:"url"`
}

type Team struct {
	ID         string `json:"id"`
	Name       string `json:"name"`
	IssueCount int    `json:"issueCount"`
}

type TeamIssue struct {
	ID          string `json:"id"`
	Title       string `json:"title"`
	Description string `json:"description"`
}

type Teams struct {
	Nodes []Team `json:"nodes"`
}

type IssueResponse struct {
	Data struct {
		Issue Issue `json:"issue"`
	} `json:"data"`
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
				Nodes []TeamIssue `json:"nodes"`
			} `json:"issues"`
		} `json:"team"`
	} `json:"data"`
}
