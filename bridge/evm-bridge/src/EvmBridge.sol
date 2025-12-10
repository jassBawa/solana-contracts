// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import "./WrappedToken.sol";

error Unauthorized();
error AlreadyProcessed(bytes32 messageId);
error ZeroAddress();
error BridgeIsPaused();
error NonceOverflow();

contract EvmBridge {
    WrappedToken public wrappedToken;

    address public admin;
    address public relayer;
    uint64 public burnNonce;
    bool public paused;

    mapping(bytes32 => bool) public processedMessages;

    // events
    event AdminUpdated(address indexed oldAdmin, address indexed newAdmin);
    event RelayerUpdated(
        address indexed oldRelayer,
        address indexed newRelayer
    );
    event BridgePaused(address indexed admin);
    event BridgeUnPaused(address indexed admin);
    event MintedFromSolana(
        bytes32 indexed messageId,
        uint64 srcChainId,
        bytes32 indexed config,
        uint64 nonce,
        bytes32 tokenMint,
        bytes32 solanaUser,
        uint256 amount,
        address recipient
    );
    event BurnedToSolana(
        bytes32 indexed messageId,
        uint64 srcChainId,
        uint64 dstChainId,
        bytes32 indexed config,
        uint64 nonce,
        uint256 amount,
        bytes32 solanaRecipient
    );

    constructor(address _admin, address _relayer) {
        require(_admin != address(0), "admin zero");
        require(_relayer != address(0), "relayer zero");

        admin = _admin;
        relayer = _relayer;

        wrappedToken = new WrappedToken(
            "Wrapped Solana Token",
            "wSOLT",
            address(this)
        );
    }

    modifier onlyRelayer() {
        if (msg.sender != relayer) revert Unauthorized();
        _;
    }

    modifier onlyAdmin() {
        if (msg.sender != admin) revert Unauthorized();
        _;
    }

    modifier whenNotPaused() {
        if (paused) revert BridgeIsPaused();
        _;
    }

    // admin functions
    function setAdmin(address _admin) external onlyAdmin {
        if (_admin == address(0)) revert ZeroAddress();
        address oldAdmin = admin;
        admin = _admin;
        emit AdminUpdated(oldAdmin, _admin);
    }

    function setRelayer(address _relayer) external onlyAdmin {
        if (_relayer == address(0)) revert ZeroAddress();
        address oldRelayer = relayer;
        relayer = _relayer;
        emit RelayerUpdated(oldRelayer, _relayer);
    }

    function pause() external onlyAdmin {
        paused = true;
        emit BridgePaused(msg.sender);
    }

    function unpause() external onlyAdmin {
        paused = false;
        emit BridgeUnPaused(msg.sender);
    }

    function mintFromSolana(
        uint64 srcChainId,
        bytes32 config,
        uint64 nonce,
        bytes32 tokenMint,
        bytes32 solanaUser,
        uint256 amount,
        address recipient
    ) external onlyRelayer whenNotPaused {
        require(amount > 0, "amount zero");
        require(recipient != address(0), "recipient zero");

        bytes32 messageId = keccak256(abi.encode(srcChainId, config, nonce));
        if (processedMessages[messageId]) revert AlreadyProcessed(messageId);

        processedMessages[messageId] = true;
        wrappedToken.mint(recipient, amount);

        emit MintedFromSolana(
            messageId,
            srcChainId,
            config,
            nonce,
            tokenMint,
            solanaUser,
            amount,
            recipient
        );
    }

    function burnWrapped(
        uint64 dstChainId,
        bytes32 config,
        uint256 amount,
        bytes32 solanaRecipient
    ) external whenNotPaused {
        require(amount > 0, "amount zero");
        require(solanaRecipient != bytes32(0), "recipient zero");

        if (burnNonce == type(uint64).max) revert NonceOverflow();
        uint64 currentNonce = burnNonce;

        // saves gas via skipping Solidity’s built-in overflow check
        unchecked {
            burnNonce++;
        }

        bytes32 messageId = keccak256(
            abi.encode(
                block.chainid,
                dstChainId,
                config,
                currentNonce,
                amount,
                solanaRecipient
            )
        );

        if (processedMessages[messageId]) revert AlreadyProcessed(messageId);
        processedMessages[messageId] = true;

        wrappedToken.burnFrom(msg.sender, amount);

        emit BurnedToSolana(
            messageId,
            uint64(block.chainid),
            dstChainId,
            config,
            currentNonce,
            amount,
            solanaRecipient
        );
    }

    function isMessageProcessed(
        bytes32 messageId
    ) external view returns (bool) {
        return processedMessages[messageId];
    }

    function getBurnNonce() external view returns (uint64) {
        return burnNonce;
    }
}
