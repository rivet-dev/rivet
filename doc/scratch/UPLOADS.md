# Uploads

## Flow

1. Call `upload-prepare`
1. Send presigned link to client
1. Client uploads files to that link
1. Call `upload-complete` and save the upload ID somewhere
	* Make sure to save the upload ID only once the upload is complete

## Making files public

Create a Traefik rule for this.

