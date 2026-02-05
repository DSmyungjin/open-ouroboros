#!/bin/bash
BIN="./target/debug/ouroboros"

echo "===1.2: Korean keyword===" && $BIN search "데이터베이스" --limit 3 2>&1 | grep -v "INFO"
echo -e "\n===1.3: Multiple keywords===" && $BIN search "API design" --limit 3 2>&1 | grep -v "INFO"
echo -e "\n===1.4: Korean multiple words===" && $BIN search "사용자 관리" --limit 3 2>&1 | grep -v "INFO"
echo -e "\n===2.1: Mixed language===" && $BIN search "REST 디자인" --limit 3 2>&1 | grep -v "INFO"
echo -e "\n===2.2: Korean morphological===" && $BIN search "검색" --limit 3 2>&1 | grep -v "INFO"
echo -e "\n===2.3: Technical term in Korean===" && $BIN search "Docker" --limit 3 2>&1 | grep -v "INFO"
echo -e "\n===3.1: Filter by type - task===" && $BIN search "API" --doc-type task --limit 3 2>&1 | grep -v "INFO"
echo -e "\n===3.2: Filter by type - result===" && $BIN search "API" --doc-type result --limit 3 2>&1 | grep -v "INFO"
echo -e "\n===3.3: Filter by type - knowledge===" && $BIN search "API" --doc-type knowledge --limit 3 2>&1 | grep -v "INFO"
echo -e "\n===3.4: Filter by session===" && $BIN search "task" --session "0db373-search-cli-evaluation" --limit 3 2>&1 | grep -v "INFO"
echo -e "\n===3.5: Combined filters===" && $BIN search "API" --doc-type task --session "0db373-search-cli-evaluation" --limit 3 2>&1 | grep -v "INFO"
echo -e "\n===4.1: Empty query===" && $BIN search "" 2>&1 | grep -v "INFO" | head -5
echo -e "\n===4.2: Non-existent term===" && $BIN search "xyznonexistent123" 2>&1 | grep -v "INFO"
echo -e "\n===4.3: Invalid doc type===" && $BIN search "API" --doc-type invalid 2>&1 | grep -v "INFO" | head -3
echo -e "\n===4.4: Special characters===" && $BIN search "@#\$" --limit 2 2>&1 | grep -v "INFO"
echo -e "\n===5.1: Help command===" && $BIN search --help 2>&1 | head -15
