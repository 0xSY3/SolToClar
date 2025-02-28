contract Counter {
    uint256 count;

    function increment() {
        count = count + 1;
    }

    function getCount() returns (uint256) {
        return count;
    }
}

contract TokenManager {
    // Simple mapping
    mapping(address => uint256) public balances;

    // Nested mapping for token approvals
    mapping(address => mapping(uint256 => bool)) public approvals;

    // Complex value type mapping
    mapping(uint256 => address) public tokenOwners;

    function approve(address owner, uint256 tokenId) public {
        approvals[owner][tokenId] = true;
    }

    function transfer(address to, uint256 amount) public {
        balances[msg.sender] = balances[msg.sender] - amount;
        balances[to] = balances[to] + amount;
    }
}