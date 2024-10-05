package main

import (
	"fmt"
	"log"
	"time"

	"github.com/aws/aws-lambda-go/lambda"
)

var countryDocumentTypeMap = map[string]string{
	"finland": "10YFI-1--------U",
	"sweden":  "10YSE-1--------K",
	"norway":  "10YNO-1--------9",
}

func handleInvocation() {
	year, month, date := time.Now().Date()

	for country, documentType := range countryDocumentTypeMap {
		pricing, err := GetElectricityPricingForMonth(documentType)
		if err != nil {
			log.Print(fmt.Errorf("Failed fetching electricity pricing: %v", err))
			return
		}

		_, err = StorePricingData(fmt.Sprint(year, "-", month.String(), "-", date), country, pricing)
		if err != nil {
			log.Fatal("Failed inserting pricing data to DynamoDB")
		}

		log.Print("Pricing data inserted to DynamoDB")
	}
}

func main() {
	lambda.Start(handleInvocation)
}
