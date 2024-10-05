package main

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"time"
)

func FetchElectricityPricing(documentType string) ([]byte, error) {
	now := time.Now()

	today := time.Date(now.Year(), now.Month(), now.Day(), 22, 0, 0, 0, now.Location()).Format("200601021504")
	firstOfCurrentMonth := time.Date(now.Year(), now.Month(), 1, 22, 0, 0, 0, now.Location())
	lastDayOfPreviousMonth := firstOfCurrentMonth.AddDate(0, 0, -1).Format("200601021504")

	baseUrl := fmt.Sprintf("https://web-api.tp.entsoe.eu/api?documentType=A44&out_Domain=%s&in_Domain=%s&periodStart=%s&periodEnd=%s&securityToken=", documentType, documentType, lastDayOfPreviousMonth, today)

	securityToken := os.Getenv("SECURITY_TOKEN")

	if len(securityToken) == 0 {
		return nil, fmt.Errorf("SECURITY_TOKEN environment variable not set")
	}

	resp, err := http.Get(fmt.Sprintf("%s%s", baseUrl, securityToken))
	if err != nil {
		return nil, fmt.Errorf("error fetching data from server")
	}

	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		fmt.Printf("Response body: %v", body)
		return nil, fmt.Errorf("Failed reading response body")
	}

	return body, nil
}
