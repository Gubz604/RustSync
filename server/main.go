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

// const backupRoot = "../../../Backups-Go"

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

		tempFile, tempErr := os.CreateTemp(parent, ".rustsync-upload-*")
		if tempErr != nil {
			http.Error(w, "Error creating temporary file", http.StatusInternalServerError)
			fmt.Println("Error creating temporary file")
			return
		}
		hasher := sha256.New()

		defer func() {
			tempFile.Close()
			os.Remove(tempFile.Name())
		}()

		writer := io.MultiWriter(tempFile, hasher)

		written, writeErr := io.Copy(writer, r.Body)
		if writeErr != nil {
			http.Error(w, "Error writing temporary file", http.StatusInternalServerError)
			fmt.Println("Error writing temporary file")
			return
		}

		if parsedSize != uint64(written) {
			http.Error(w, "File size does not match size in header", http.StatusBadRequest)
			fmt.Println("File size does not match size in header")
			return
		}

		hashString := fmt.Sprintf("%x", hasher.Sum(nil))
		if hashString != hash {
			http.Error(w, "Hash from metadata does not match file hash", http.StatusBadRequest)
			fmt.Println("Hash from metadata does not match file hash")
			return
		}

		closeErr := tempFile.Close()
		if closeErr != nil {
			http.Error(w, "Error in closing temporary file", http.StatusInternalServerError)
			fmt.Println("Error in closing temporary file")
			return
		}

		renameErr := os.Rename(tempFile.Name(), destination)
		if renameErr != nil {
			http.Error(w, "Error in renaming temporary file", http.StatusInternalServerError)
			fmt.Println("Error in renaming temporary file")
			return
		}


		fmt.Printf("Received %d bytes\n", written)
		fmt.Printf("File Path: %s\n", cleanedPath)
		fmt.Printf("File Size: %s bytes\n", r.Header.Get("X-File-Size"))
		fmt.Printf("File Hash: %s\n", r.Header.Get("X-File-Hash"))

		w.WriteHeader(http.StatusOK)
		fmt.Fprint(w, "OK")
	}
}

func main() {
	fmt.Println("RustSync Backup Server")

	if len(os.Args) != 2 {
		fmt.Println("Usage <Backup Destination Path>")
		return
	}

	backupRoot := os.Args[1]
	cleanedBackupRoot := filepath.Clean(backupRoot)
	backupAbs, backupAbsErr := filepath.Abs(cleanedBackupRoot)
	if backupAbsErr != nil {
		fmt.Printf("Failed to create the backup root as an Absolute Path %v\n", backupAbsErr)
		return
	}
	mkdirErr := os.MkdirAll(backupAbs, 0755)
	if mkdirErr != nil {
		fmt.Printf("Backup directory failed to be created %v\n", mkdirErr)
		return
	}

	fmt.Printf("---- Backup Directory: %s ----\n", backupAbs)

	http.HandleFunc("/health", healthHandler)

	http.HandleFunc("/upload", makeUploadHandler(backupAbs))

	fmt.Println("Listening on :8080")
	if err := http.ListenAndServe(":8080", nil); err != nil {
		fmt.Println(err)
	}
}