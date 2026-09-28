package main

import (
	"crypto/sha256"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

const backupRoot = "../../../Backups-Go"

func healthHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodGet {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}
	
	w.WriteHeader(http.StatusOK)
	fmt.Fprint(w, "OK")
}

func makeUploadHandler(backupRoot string) http.HandlerFunc {
	return func (w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
			fmt.Println("Method not allowed")
			return 
		}

		path := r.Header.Get("X-File-Path")
		size := r.Header.Get("X-File-Size")
		hash := r.Header.Get("X-File-Hash")

		if path == "" || size == "" || hash == "" {
			http.Error(w, "Metadata not correctly received", http.StatusBadRequest)
			fmt.Println("Metadata not correctly received")
			return
		}

		parsedSize, sizeErr := strconv.ParseUint(size, 10, 64)
		if sizeErr != nil {
			http.Error(w, "File Size received not an integer value", http.StatusBadRequest)
			fmt.Println("File Size received not an integer value")
			return
		}

		cleanedPath := filepath.Clean(path)
		if filepath.IsAbs(cleanedPath) {
			http.Error(w, "Filepath received is an absolute path", http.StatusBadRequest)
			fmt.Println("Filepath received is an absolute path")
			return
		}

		prefix := ".." + string(filepath.Separator)
		if strings.HasPrefix(cleanedPath, prefix) || cleanedPath == ".." {
			http.Error(w, "Invalid filepath", http.StatusBadRequest)
			fmt.Println("Invalid filepath")
			return
		}

		data, err := io.ReadAll(r.Body)
		if err != nil {
			http.Error(w,"Error reading bytes from upload", http.StatusInternalServerError)
			fmt.Println("Error reading bytes from upload")
			return
		}

		if parsedSize != uint64(len(data)) {
			http.Error(w, "File size does not match size in header", http.StatusBadRequest)
			fmt.Println("File size does not match size in header")
			return
		}

		hashString := fmt.Sprintf("%x", sha256.Sum256(data))
		if hashString != hash {
			http.Error(w, "Hash from metadata does not match file hash", http.StatusBadRequest)
			fmt.Println("Hash from metadata does not match file hash")
			return
		}

		destination := filepath.Join(backupRoot, cleanedPath)

		absoluteRoot, absRootErr := filepath.Abs(backupRoot)
		if absRootErr != nil {
			http.Error(w, "Error producing the absolute root", http.StatusInternalServerError)
			fmt.Println("Error producing the absolute root")
			return
		}
		absoluteDestination, absDestinationErr := filepath.Abs(destination)
		if absDestinationErr != nil {
			http.Error(w, "Error producing the absolute destination", http.StatusInternalServerError)
			fmt.Println("Error producing the absolute destination")
			return
		}
		relativeDestination, relativeDesErr := filepath.Rel(absoluteRoot, absoluteDestination)
		if relativeDesErr != nil {
			http.Error(w, "Error in creating a relative destination", http.StatusInternalServerError)
			fmt.Println("Error in creating a relative destination")
			return
		}
		
		if strings.HasPrefix(relativeDestination, prefix) || relativeDestination == ".." {
			http.Error(w, "Destination is outside of root", http.StatusBadRequest)
			fmt.Println("Destination is outside of root")
			return
		}

		parent := filepath.Dir(destination)
		mkdirErr := os.MkdirAll(parent, 0755)
		if mkdirErr != nil {
			http.Error(w, "Destination directory failed to be created", http.StatusInternalServerError)
			fmt.Println("Destination directory failed to be created")
			return
		}

		writeFileErr := os.WriteFile(destination, data, 0644)
		if writeFileErr != nil {
			http.Error(w, "File failed to be written", http.StatusInternalServerError)
			fmt.Println("File failed to be written")
			return
		}

		fmt.Printf("Received %d bytes\n", len(data))
		fmt.Printf("File Path: %s\n", cleanedPath)
		fmt.Printf("File Size: %s bytes\n", r.Header.Get("X-File-Size"))
		fmt.Printf("File Hash: %s\n", r.Header.Get("X-File-Hash"))

		w.WriteHeader(http.StatusOK)
		fmt.Fprint(w, "OK")
	}
}

func main() {
	fmt.Println("RustSync Backup Server")

	http.HandleFunc("/health", healthHandler)

	http.HandleFunc("/upload", makeUploadHandler(backupRoot))

	fmt.Println("Listening on :8080")
	if err := http.ListenAndServe(":8080", nil); err != nil {
		fmt.Println(err)
	}
}