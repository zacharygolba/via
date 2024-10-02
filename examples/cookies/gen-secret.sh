#!/bin/sh
#
# Generates a new secret key and prints it to stdout.
#

set -e

if [[ "$OSTYPE" == "darwin"* ]]; then
    secret="$(openssl rand -base64 64 | sed -e ':a' -e 'N' -e '$!ba' -e 's/\n//g')"
else
    secret="$(openssl rand -base64 64 | sed ':a;N;$!ba;s/\n//g')"
fi

echo
echo "  Successfully generated a new secret key."
echo
echo "  Replace the value of VIA_SECRET_KEY in your .env file with the following:"
echo
echo "\"$secret\""
echo
