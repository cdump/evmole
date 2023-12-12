// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

contract Example {
    uint256 public balance;

    function deposit() public payable {
        balance += msg.value;
    }

    function inc(uint32 v) public {
        balance += v;
    }
}
