#!/bin/bash

set -e

release-plz release-pr --git-token="$GITHUB_TOKEN"
