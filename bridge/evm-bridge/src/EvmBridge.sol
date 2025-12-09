// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;


contract WrappedToken {
    string public name = "Wrapped Solana Token";
    string public symbol = "wSOLT";
    uint8 public decimals = 18;

    uint256 public totalSupply;
    address public minter;

    mapping(address => uint256) public balanceOf;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event MinterUpdated(address indexed newMinter);

    modifier onlyMinter() {
        require(msg.sender == minter, "Not minter");
        _;
    }

    constructor(address _minter) {
        minter = _minter;
    }

    function setMinter(address _minter) external onlyMinter {
        require(_minter != address(0), "zero address");
        minter = _minter;
        emit MinterUpdated(_minter);
    }

    function mint(address to, uint256 amount) external onlyMinter {
        require(to != address(0), "zero address");
        totalSupply += amount;
        balanceOf[to] += amount;
        emit Transfer(address(0), to, amount);
    }
}


contract EvmBridge {
    WrappedToken public wrappedToken;

    address public admin;
    address public relayer;

    /// Replay protection
    mapping(bytes32 => bool) public processedMessages;

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

    error Unauthorized();
    error AlreadyProcessed(bytes32 messageId);

    constructor(address _admin, address _relayer) {
        require(_admin != address(0), "admin zero");
        require(_relayer != address(0), "relayer zero");

        admin = _admin;
        relayer = _relayer;

        // Deploy wrapped token and make this bridge the minter
        wrappedToken = new WrappedToken(address(this));
    }

    modifier onlyAdmin() {
        if (msg.sender != admin) revert Unauthorized();
        _;
    }

    modifier onlyRelayer() {
        if (msg.sender != relayer) revert Unauthorized();
        _;
    }

    function setRelayer(address _relayer) external onlyAdmin {
        require(_relayer != address(0), "zero address");
        relayer = _relayer;
    }


    function mintFromSolana(
        uint64 srcChainId,
        bytes32 config,
        uint64 nonce,
        bytes32 tokenMint,
        bytes32 solanaUser,
        uint256 amount,
        address recipient
    ) external onlyRelayer {
        require(amount > 0, "amount zero");
        require(recipient != address(0), "recipient zero");

        bytes32 messageId = keccak256(
            abi.encode(
                srcChainId,
                config,
                nonce
            )
        );

        if (processedMessages[messageId]) {
            revert AlreadyProcessed(messageId);
        }

        processedMessages[messageId] = true;

        // Mint wrapped tokens
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
}

