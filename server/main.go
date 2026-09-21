package main

import (
	"fmt"
	"net/http"
)

func healthHandler(w http.ResponseWriter, r *http.Request) {
	if !(r.Method == http.MethodGet) {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	} else {
		w.WriteHeader(http.StatusOK)
		fmt.Fprint(w, "OK")
	}
}

func main() {
	fmt.Println("RustSync Backup Server")

	http.HandleFunc("/health", healthHandler)

	fmt.Println("Listening on :8080")
	if err := http.ListenAndServe(":8080", nil); err != nil {
		fmt.Println(err)
	}
}