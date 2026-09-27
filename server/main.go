package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"io"
	"strconv"
	"crypto/sha256"
)

type Backup struct {
	Path string `json:"path"`
	Size uint64 `json:"size"`
	Hash string `json:"hash"`
}

func healthHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	} else {
		w.WriteHeader(http.StatusOK)
		fmt.Fprint(w, "OK")
	}
}

func backupHandler(w http.ResponseWriter, r *http.Request) {
	var backup Backup

	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	} 

	decoder := json.NewDecoder(r.Body)

	if err := decoder.Decode(&backup); err != nil {
		http.Error(w, "Invalid JSON", http.StatusBadRequest)
		return
	}

	fmt.Println(backup)

	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "OK")
}

func uploadHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return 
	}

	path := r.Header.Get("X-File-Path")
	size := r.Header.Get("X-File-Size")
	hash := r.Header.Get("X-File-Hash")

	if path == "" || size == "" || hash == "" {
		http.Error(w, "Metadata not correctly received", http.StatusBadRequest)
		return
	}

	parsedSize, sizeErr := strconv.ParseUint(size, 10, 64)
	if sizeErr != nil {
		http.Error(w, "File Size received not an integer value", http.StatusBadRequest)
		return
	}

	data, err := io.ReadAll(r.Body)
	if err != nil {
		http.Error(w,"Error reading bytes from upload", http.StatusInternalServerError)
		return
	}

	if parsedSize != uint64(len(data)) {
		http.Error(w, "File size does not match size in header", http.StatusBadRequest)
		return
	}

	hashString := fmt.Sprintf("%x", sha256.Sum256(data))
	if hashString != hash {
		http.Error(w, "Hash from metadata does not match file hash", http.StatusBadRequest)
		return
	}

	fmt.Printf("Received %d bytes\n", len(data))
	fmt.Printf("File Path: %s\n", r.Header.Get("X-File-Path"))
	fmt.Printf("File Size: %s bytes\n", r.Header.Get("X-File-Size"))
	fmt.Printf("File Hash: %s\n", r.Header.Get("X-File-Hash"))

	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "OK")
}

func main() {
	fmt.Println("RustSync Backup Server")

	http.HandleFunc("/health", healthHandler)

	http.HandleFunc("/backup", backupHandler)

	http.HandleFunc("/upload", uploadHandler)

	fmt.Println("Listening on :8080")
	if err := http.ListenAndServe(":8080", nil); err != nil {
		fmt.Println(err)
	}
}