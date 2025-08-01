// SPDX-License-Identifier: Apache-2.0

pragma solidity ^0.8.20;

contract AcceptingShielder {
    function withdrawNative(
        bytes3 expectedContractVersion,
        uint256 amount,
        address withdrawalAddress,
        uint256 merkleRoot,
        uint256 oldNullifierHash,
        uint256 newNote,
        bytes calldata proof,
        address relayerAddress,
        uint256 relayerFee,
        uint256 macSalt,
        uint256 macCommitment,
        bytes calldata memo
    ) external {}

    function withdrawERC20(
        bytes3 expectedContractVersion,
        address tokenAddress,
        uint256 amount,
        address withdrawalAddress,
        uint256 merkleRoot,
        uint256 oldNullifierHash,
        uint256 newNote,
        bytes calldata proof,
        address relayerAddress,
        uint256 relayerFee,
        uint256 macSalt,
        uint256 macCommitment,
        bytes calldata memo
    ) external payable {}
}
