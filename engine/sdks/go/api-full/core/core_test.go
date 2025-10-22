package core

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"strconv"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestCase represents a single test case.
type TestCase struct {
	description string

	// Server-side assertions.
	giveMethod             string
	giveResponseIsOptional bool
	giveHeader             http.Header
	giveErrorDecoder       ErrorDecoder
	giveRequest            *Request

	// Client-side assertions.
	wantResponse *Response
	wantError    error
}

// Request a simple request body.
type Request struct {
	Id string `json:"id"`
}

// Response a simple response body.
type Response struct {
	Id string `json:"id"`
}

// NotFoundError represents a 404.
type NotFoundError struct {
	*APIError

	Message string `json:"message"`
}

func TestCall(t *testing.T) {
	tests := []*TestCase{
		{
			description: "GET success",
			giveMethod:  http.MethodGet,
			giveHeader: http.Header{
				"X-API-Status": []string{"success"},
			},
			giveRequest: &Request{
				Id: "123",
			},
			wantResponse: &Response{
				Id: "123",
			},
		},
		{
			description: "GET not found",
			giveMethod:  http.MethodGet,
			giveHeader: http.Header{
				"X-API-Status": []string{"fail"},
			},
			giveRequest: &Request{
				Id: strconv.Itoa(http.StatusNotFound),
			},
			giveErrorDecoder: newTestErrorDecoder(t),
			wantError: &NotFoundError{
				APIError: NewAPIError(
					http.StatusNotFound,
					errors.New(`{"message":"ID \"404\" not found"}`),
				),
			},
		},
		{
			description: "POST optional response",
			giveMethod:  http.MethodPost,
			giveHeader: http.Header{
				"X-API-Status": []string{"success"},
			},
			giveRequest: &Request{
				Id: "123",
			},
			giveResponseIsOptional: true,
		},
		{
			description: "POST API error",
			giveMethod:  http.MethodPost,
			giveHeader: http.Header{
				"X-API-Status": []string{"fail"},
			},
			giveRequest: &Request{
				Id: strconv.Itoa(http.StatusInternalServerError),
			},
			wantError: NewAPIError(
				http.StatusInternalServerError,
				errors.New("failed to process request"),
			),
		},
	}
	for _, test := range tests {
		t.Run(test.description, func(t *testing.T) {
			var (
				server = newTestServer(t, test)
				client = server.Client()
			)
			caller := NewCaller(client)
			var response *Response
			err := caller.Call(
				context.Background(),
				&CallParams{
					URL:                server.URL,
					Method:             test.giveMethod,
					Headers:            test.giveHeader,
					Request:            test.giveRequest,
					Response:           &response,
					ResponseIsOptional: test.giveResponseIsOptional,
					ErrorDecoder:       test.giveErrorDecoder,
				},
			)
			if test.wantError != nil {
				assert.EqualError(t, err, test.wantError.Error())
				return
			}
			require.NoError(t, err)
			assert.Equal(t, test.wantResponse, response)
		})
	}
}

// newTestServer returns a new *httptest.Server configured with the
// given test parameters.
func newTestServer(t *testing.T, tc *TestCase) *httptest.Server {
	return httptest.NewServer(
		http.HandlerFunc(
			func(w http.ResponseWriter, r *http.Request) {
				assert.Equal(t, tc.giveMethod, r.Method)
				assert.Equal(t, contentType, r.Header.Get(contentTypeHeader))
				for header, value := range tc.giveHeader {
					assert.Equal(t, value, r.Header.Values(header))
				}

				bytes, err := io.ReadAll(r.Body)
				require.NoError(t, err)

				request := new(Request)
				require.NoError(t, json.Unmarshal(bytes, request))

				switch request.Id {
				case strconv.Itoa(http.StatusNotFound):
					notFoundError := &NotFoundError{
						APIError: &APIError{
							StatusCode: http.StatusNotFound,
						},
						Message: fmt.Sprintf("ID %q not found", request.Id),
					}
					bytes, err = json.Marshal(notFoundError)
					require.NoError(t, err)

					w.WriteHeader(http.StatusNotFound)
					_, err = w.Write(bytes)
					require.NoError(t, err)
					return

				case strconv.Itoa(http.StatusInternalServerError):
					w.WriteHeader(http.StatusInternalServerError)
					_, err = w.Write([]byte("failed to process request"))
					require.NoError(t, err)
					return
				}

				if tc.giveResponseIsOptional {
					w.WriteHeader(http.StatusOK)
					return
				}

				response := &Response{
					Id: request.Id,
				}
				bytes, err = json.Marshal(response)
				require.NoError(t, err)

				_, err = w.Write(bytes)
				require.NoError(t, err)
			},
		),
	)
}

// newTestErrorDecoder returns an error decoder suitable for tests.
func newTestErrorDecoder(t *testing.T) func(int, io.Reader) error {
	return func(statusCode int, body io.Reader) error {
		raw, err := io.ReadAll(body)
		require.NoError(t, err)

		var (
			apiError = NewAPIError(statusCode, errors.New(string(raw)))
			decoder  = json.NewDecoder(bytes.NewReader(raw))
		)
		switch statusCode {
		case 404:
			value := new(NotFoundError)
			value.APIError = apiError
			require.NoError(t, decoder.Decode(value))

			return value
		}
		return apiError
	}
}
