package main

import (
	"crypto/sha256"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"fmt"
	"bytes"
	"path/filepath"
	"os"
)

func TestUploadRejectsWrongMethod(t *testing.T) {
	req := httptest.NewRequest(http.MethodGet, "/path/example", nil)
	recorder := httptest.NewRecorder()

	tempDir := t.TempDir()
	handler := makeUploadHandler(tempDir)
	handler(recorder, req)

	expected := http.StatusMethodNotAllowed

	if recorder.Code != expected {
		t.Errorf("expected %d, got %d", expected, recorder.Code)
	}

	if !strings.Contains(recorder.Body.String(), "Method not allowed") {
		t.Errorf("unexpected response body: %s", recorder.Body.String())
	}
}

func TestUploadRejectsMissingMetadata(t *testing.T) {
	req := httptest.NewRequest(http.MethodPost, "/path/example", nil)
	req.Header.Set("X-File-Path", "")
	recorder := httptest.NewRecorder()

	tempDir := t.TempDir()
	handler := makeUploadHandler(tempDir)
	handler(recorder, req)

	expected := http.StatusBadRequest

	if recorder.Code != expected {
		t.Errorf("expected %d, got %d", expected, recorder.Code)
	}

	if !strings.Contains(recorder.Body.String(), "Metadata not correctly received") {
		t.Errorf("unexpected response body: %s", recorder.Body.String())
	}
}

func TestUploadRejectsPathTraversal(t *testing.T) {
	req := httptest.NewRequest(http.MethodPost, "/path/example", nil)
	req.Header.Set("X-File-Path", "../outside.txt")
	req.Header.Set("X-File-Size", "10")
	req.Header.Set("X-File-Hash", "This_is_a_hash")
	recorder := httptest.NewRecorder()

	tempDir := t.TempDir()
	handler := makeUploadHandler(tempDir)
	handler(recorder, req)

	expected := http.StatusBadRequest

	if recorder.Code != expected {
		t.Errorf("expected %d, got %d", expected, recorder.Code)
	}

	if !strings.Contains(recorder.Body.String(), "Invalid filepath") {
		t.Errorf("unexpected response body: %s", recorder.Body.String())
	}
}

func TestUploadRejectsAbsolutePath(t *testing.T) {
	req := httptest.NewRequest(http.MethodPost, "/path/example", nil)
	req.Header.Set("X-File-Path", "C:/path/to/example.txt")
	req.Header.Set("X-File-Size", "10")
	req.Header.Set("X-File-Hash", "This_is_a_hash")
	recorder := httptest.NewRecorder()

	tempDir := t.TempDir()
	handler := makeUploadHandler(tempDir)
	handler(recorder, req)

	expected := http.StatusBadRequest

	if recorder.Code != expected {
		t.Errorf("expected status %d, got %d", expected, recorder.Code)
	}

	if !strings.Contains(recorder.Body.String(), "Filepath received is an absolute path") {
		t.Errorf("unexpected response body: %s", recorder.Body.String())
	}
}

func TestUploadRejectsIncorrectSize(t *testing.T) {
	body := strings.NewReader("hello")
	req := httptest.NewRequest(http.MethodPost, "/path/example", body)
	req.Header.Set("X-File-Path", "example.txt")
	req.Header.Set("X-File-Size", "10")
	req.Header.Set("X-File-Hash", "placeholder")
	recorder := httptest.NewRecorder()

	tempDir := t.TempDir()
	handler := makeUploadHandler(tempDir)
	handler(recorder, req)

	expected := http.StatusBadRequest

	if recorder.Code != expected {
		t.Errorf("expected status %d, got %d", expected, recorder.Code)
	}

	if !strings.Contains(recorder.Body.String(), "File size does not match size in header") {
		t.Errorf("unexpected response body: %s", recorder.Body.String())
	}
}

func TestUploadRejectsIncorrectHash(t *testing.T) {
	body := strings.NewReader("hello")
	req := httptest.NewRequest(http.MethodPost, "/path/example", body)
	req.Header.Set("X-File-Path", "example.txt")
	req.Header.Set("X-File-Size", "5")
	req.Header.Set("X-File-Hash", "definitelynotthecorrecthash")
	recorder := httptest.NewRecorder()

	tempDir := t.TempDir()
	handler := makeUploadHandler(tempDir)
	handler(recorder, req)

	expected := http.StatusBadRequest

	if recorder.Code != expected {
		t.Errorf("expected status %d, got %d", expected, recorder.Code)
	}

	if !strings.Contains(recorder.Body.String(), "Hash from metadata does not match file hash") {
		t.Errorf("unexpected response body: %s", recorder.Body.String())
	}
}

func TestUploadSucceeds(t *testing.T) {

	data := []byte("hello")
	sum := sha256.Sum256(data)
	hash := fmt.Sprintf("%x", sum)

	req := httptest.NewRequest(http.MethodPost, "/path/example", bytes.NewReader(data))
	req.Header.Set("X-File-Path", "nested/example.txt")
	req.Header.Set("X-File-Size", "5")
	req.Header.Set("X-File-Hash", hash)
	recorder := httptest.NewRecorder()

	tempDir := t.TempDir()
	handler := makeUploadHandler(tempDir)
	handler(recorder, req)

	expected := http.StatusOK
	if recorder.Code != expected {
		t.Errorf("expected status %d, got %d", expected, recorder.Code)
	}

	destination := filepath.Join(tempDir, "nested", "example.txt")

	storedData, err := os.ReadFile(destination)
	if err != nil {
		t.Fatalf("failed to read uploaded file: %v", err)
	}

	if !bytes.Equal(storedData, data) {
		t.Errorf("stored file contents do not match uploaded contents")
	}
}