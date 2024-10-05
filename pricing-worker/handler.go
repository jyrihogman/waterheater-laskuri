package main

import (
	"encoding/json"
	"encoding/xml"
	"fmt"
	"log"
	"strings"
	"time"
)

func GetElectricityPricingForMonth(documentType string) ([]PricePerHour, error) {
	body, err := FetchElectricityPricing(documentType)
	if err != nil {
		return nil, err
	}

	pricingData, err := parseResponseBody(body)
	if err != nil {
		return nil, err
	}

	return pricingData, nil
}

func parseResponseBody(body []byte) ([]PricePerHour, error) {
	var pricing Publication_Marketdocument
	err := xml.Unmarshal(body, &pricing)
	if err != nil {
		return nil, fmt.Errorf("failed parsing XML")
	}

	jsonData, err := json.MarshalIndent(pricing, "", "  ")
	if err != nil {
		return nil, fmt.Errorf("error marshalling to JSON: %v", err)
	}

	log.Print(string(jsonData))

	pricingData, err := mapPricesToHours(jsonData)
	if err != nil {
		return nil, fmt.Errorf("error marshalling to JSON: %v", err)
	}

	return pricingData, nil
}

func mapPricesToHours(bytes []byte) ([]PricePerHour, error) {
	var doc Publication_Marketdocument
	err := json.Unmarshal(bytes, &doc)
	if err != nil {
		log.Fatal(err)
		return nil, fmt.Errorf("error unmarshalling JSON: %v", err)
	}

	pricePerHour := []PricePerHour{}
	currentMonth := time.Now().Month()

	for _, period := range doc.TimeSeries.Period {
		startTimeString := strings.TrimSpace(period.TimeInterval.Start)
		startTime, err := time.Parse("2006-01-02T15:04Z", startTimeString)
		if err != nil {
			log.Fatal(err)
			return nil, fmt.Errorf("error parsing start time: %v", err)
		}

		points := fillMissingPositions(period.Point)

		for _, point := range points {
			timeStamp := startTime.Add(time.Duration(point.Position-1) * time.Hour)

			if timeStamp.Month() != currentMonth {
				continue
			}

			priceInfo := PricePerHour{
				TimeStamp: timeStamp,
				Price:     ((point.Price * 0.1) * 1.255),
			}
			pricePerHour = append(pricePerHour, priceInfo)
		}

	}

	return pricePerHour, nil
}

func fillMissingPositions(points []Point) []Point {
	filledPoints := []Point{}

	for i := 0; i < len(points); i++ {
		filledPoints = append(filledPoints, points[i])

		// If this isn't the last element, check for gaps in the position
		if i < len(points)-1 {
			currentPosition := points[i].Position
			nextPosition := points[i+1].Position

			// If there's a gap, fill it by inserting points with the same price as the current one
			if nextPosition > currentPosition+1 {
				for pos := currentPosition + 1; pos < nextPosition; pos++ {
					filledPoints = append(filledPoints, Point{
						Position: pos,
						Price:    points[i].Price, // Use the price of the previous point
					})
				}
			}
		}
	}

	return filledPoints
}
