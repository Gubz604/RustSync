package main

import (
	"encoding/json"
	"fmt"
	"net/http"
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

func main() {
	fmt.Println("RustSync Backup Server")

	http.HandleFunc("/health", healthHandler)

	http.HandleFunc("/backup", backupHandler)

	fmt.Println("Listening on :8080")
	if err := http.ListenAndServe(":8080", nil); err != nil {
		fmt.Println(err)
	}
}