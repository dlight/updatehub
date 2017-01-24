package main

import (
	"errors"
	"testing"

	"github.com/stretchr/testify/assert"
)

type StateTestController struct {
	EasyFota

	updateAvailable  bool
	fetchUpdateError error
}

func (c *StateTestController) CheckUpdate() bool {
	return c.updateAvailable
}

func (c *StateTestController) FetchUpdate() error {
	return c.fetchUpdateError
}

func TestStateUpdateCheck(t *testing.T) {
	testCases := []struct {
		Name         string
		Controller   *StateTestController
		InitialState State
		NextState    State
	}{
		{
			"UpdateAvailable",
			&StateTestController{updateAvailable: true},
			NewUpdateCheckState(),
			&UpdateFetchState{},
		},

		{
			"UpdateNotAvailable",
			&StateTestController{updateAvailable: false},
			NewUpdateCheckState(),
			&PollState{},
		},
	}

	for _, tc := range testCases {
		t.Run(tc.Name, func(t *testing.T) {
			fota := &EasyFota{
				state:      tc.InitialState,
				Controller: tc.Controller,
			}

			next, _ := fota.state.Handle(fota)

			assert.IsType(t, tc.NextState, next)
		})
	}
}

func TestStateUpdateFetch(t *testing.T) {
	testCases := []struct {
		Name         string
		Controller   *StateTestController
		InitialState State
		NextState    State
	}{
		{
			"WithoutError",
			&StateTestController{fetchUpdateError: nil},
			NewUpdateFetchState(),
			&InstallUpdateState{},
		},

		{
			"WithError",
			&StateTestController{fetchUpdateError: errors.New("fetch error")},
			NewUpdateFetchState(),
			&UpdateFetchState{},
		},
	}

	for _, tc := range testCases {
		t.Run(tc.Name, func(t *testing.T) {
			fota := tc.Controller
			fota.EasyFota.state = tc.InitialState
			fota.Controller = tc.Controller

			next, _ := fota.state.Handle(&fota.EasyFota)

			assert.IsType(t, tc.NextState, next)
		})
	}
}