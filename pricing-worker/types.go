package main

import "time"

type MyEvent struct{}

type TimeInterval struct {
	Start string `xml:"start"`
	End   string `xml:"end"`
}

type Point struct {
	Position int     `xml:"position"`
	Price    float32 `xml:"price.amount"`
}

type Period struct {
	TimeInterval TimeInterval `xml:"timeInterval"`
	Point        []Point      `xml:"Point"`
}

type TimeSeries struct {
	Period []Period `xml:"Period"`
}

type Publication_Marketdocument struct {
	TimeSeries TimeSeries `xml:"TimeSeries"`
}

type PricePerHour struct {
	TimeStamp time.Time `json:"date" dynamodbav:"date"` // Partition Key (as defined in table schema)
	Price     float32   `json:"price" dynamodbav:"price"`
}
