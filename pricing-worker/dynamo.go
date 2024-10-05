package main

import (
	"log"

	"github.com/aws/aws-sdk-go/aws"
	"github.com/aws/aws-sdk-go/aws/session"
	"github.com/aws/aws-sdk-go/service/dynamodb"
	"github.com/aws/aws-sdk-go/service/dynamodb/dynamodbattribute"
)

type Price struct {
	Hour  int     `json:"hour"`
	Value float64 `json:"value"`
}

type Row struct {
	Date    string         `json:"date"`
	Country string         `json:"country"`
	Pricing []PricePerHour `json:"pricing"`
}

func getDynamoDbClient() *dynamodb.DynamoDB {
	sess := session.Must(session.NewSessionWithOptions(session.Options{
		SharedConfigState: session.SharedConfigEnable,
	}))

	return dynamodb.New(sess)
}

func StorePricingData(date string, country string, items []PricePerHour) (*dynamodb.PutItemOutput, error) {
	client := getDynamoDbClient()

	pricing_row := Row{
		Country: country,
		Date:    date,
		Pricing: items,
	}

	attributeValues, err := dynamodbattribute.MarshalMap(pricing_row)
	if err != nil {
		log.Fatalf("Got error marshalling new movie item: %s", err)
		return nil, err
	}

	putItemInput := dynamodb.PutItemInput{
		Item:      attributeValues,
		TableName: aws.String("electricity_monthly_pricing"),
	}

	result, err := client.PutItem(&putItemInput)
	if err != nil {
		log.Fatalf("Got error calling PutItem: %s", err)
		return nil, err

	}

	return result, nil
}
