// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.9;

import "./Verifier.sol";
import "@openzeppelin/contracts/utils/Strings.sol";

contract <%RULE_INDEX%>VerifierWrapper {
    using Strings for uint;
    uint constant HEADER_MAX_BYTE_SIZE = <%HEADER_MAX_BYTE_SIZE%>;
    uint constant BODY_MAX_BYTE_SIZE = <%BODY_MAX_BYTE_SIZE%>;
    address verifier;

    struct Param {
        bytes32 headerHash;
        bytes publicKey;
        uint bodyHashStart;
        string bodyHashString;
        uint fromAddressStart;
        string fromAddressString;
        uint toAddressStart;
        string toAddressString;
        uint subjectStart;
        uint manipulationIdUint;
        <%BODY_PARAM_DEFS%>
    }

    constructor(address _verifier) {
        verifier = _verifier;
    }

    function _verifyWrap(
        Param memory param,
        bytes calldata acc,
        bytes calldata proof
    ) internal view returns (bool) {
        bytes memory publicInputBytes = convertParamToBytes(param);
        bytes32 publicHash = sha256(publicInputBytes);
        uint[] memory pubInputs = new uint[](13);
        uint[12] memory accInputs = abi.decode(acc, (uint[12]));
        for (uint i = 0; i < 12; i++) {
            pubInputs[i] = accInputs[i];
        }
        uint coeff = 1;
        pubInputs[12] = 0;
        for (uint i = 0; i < 31; i++) {
            pubInputs[12] += (coeff * uint(uint8(publicHash[i])));
            coeff = coeff << 8;
        }
        return <%RULE_INDEX%>Verifier(verifier).verify(pubInputs, proof);
    }

    function convertParamToBytes(
        Param memory param
    ) private pure returns (bytes memory) {
        bytes memory metaBytes = abi.encodePacked(
            abi.encodePacked(param.headerHash),
            bytes(param.bodyHashString),
            new bytes(128 - 32 - 44),
            param.publicKey
        );
        bytes memory maskedStrPart = new bytes(
            HEADER_MAX_BYTE_SIZE + BODY_MAX_BYTE_SIZE
        );
        bytes memory substrIdsPart = new bytes(
            HEADER_MAX_BYTE_SIZE + BODY_MAX_BYTE_SIZE
        );
        bytes memory bodyHashBytes = bytes(param.bodyHashString);
        for (uint i = 0; i < bodyHashBytes.length; i++) {
            maskedStrPart[param.bodyHashStart + i] = bodyHashBytes[i];
            substrIdsPart[param.bodyHashStart + i] = bytes1(uint8(1));
        }
        bytes memory fromAddressBytes = bytes(param.fromAddressString);
        for (uint i = 0; i < fromAddressBytes.length; i++) {
            maskedStrPart[param.fromAddressStart + i] = fromAddressBytes[i];
            substrIdsPart[param.fromAddressStart + i] = bytes1(uint8(2));
        }
        bytes memory toAddressBytes = bytes(param.toAddressString);
        for (uint i = 0; i < toAddressBytes.length; i++) {
            maskedStrPart[param.toAddressStart + i] = toAddressBytes[i];
            substrIdsPart[param.toAddressStart + i] = bytes1(uint8(3));
        }
        bytes memory subjectBytes = bytes(
            string.concat(
                "Email Wallet Manipulation ID ",
                param.manipulationIdUint.toString()
            )
        );
        for (uint i = 0; i < subjectBytes.length; i++) {
            maskedStrPart[param.subjectStart + i] = subjectBytes[i];
            substrIdsPart[param.subjectStart + i] = bytes1(uint8(4));
        }

        <%BODY_SUBSTR_PARTS%>
        return abi.encodePacked(metaBytes, maskedStrPart, substrIdsPart);
    }

    function decString(
        uint intPart,
        uint decNumZero,
        uint decPart
    ) internal pure returns (string memory) {
        string memory decString = string.concat(intPart.toString(), ".");
        for (uint i = 0; i < decNumZero; i++) {
            decString = string.concat(decString, "0");
        }
        decString = string.concat(decString, decPart.toString());
        return decString;
    }
}
