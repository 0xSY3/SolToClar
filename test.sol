contract Counter {
    uint256 count;

    function increment() {
        count = count + 1;
    }

    function getCount() returns (uint256) {
        return count;
    }
}
