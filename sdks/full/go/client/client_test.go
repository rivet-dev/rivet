package client

import (
	"net/http"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func TestNewClient(t *testing.T) {
	t.Run("default", func(t *testing.T) {
		c := NewClient()
		assert.Empty(t, c.baseURL)
	})

	t.Run("base url", func(t *testing.T) {
		c := NewClient(
			WithBaseURL("test.co"),
		)
		assert.Equal(t, "test.co", c.baseURL)
	})

	t.Run("http client", func(t *testing.T) {
		httpClient := &http.Client{
			Timeout: 5 * time.Second,
		}
		c := NewClient(
			WithHTTPClient(httpClient),
		)
		assert.Empty(t, c.baseURL)
	})

	t.Run("http header", func(t *testing.T) {
		header := make(http.Header)
		header.Set("X-API-Tenancy", "test")
		c := NewClient(
			WithHTTPHeader(header),
		)
		assert.Empty(t, c.baseURL)
		assert.Equal(t, "test", c.header.Get("X-API-Tenancy"))
	})
}
