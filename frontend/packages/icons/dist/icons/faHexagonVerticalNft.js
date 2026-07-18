var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHexagonVerticalNft.js
var require_faHexagonVerticalNft = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHexagonVerticalNft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hexagon-vertical-nft";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e505";
    var svgPathData = "M260.9 495.5c-22.3 12.9-49.7 12.9-72 0L36 407.2C13.7 394.4 0 370.6 0 344.9L0 168.3c0-25.7 13.7-49.5 36-62.4L188.9 17.6c22.3-12.9 49.7-12.9 72 0l152.9 88.3c22.3 12.9 36 36.6 36 62.4l0 176.6c0 25.7-13.7 49.5-36 62.4L260.9 495.5zM95.8 170.6c-2.8-7.1-10.3-11.2-17.9-9.8s-13 8-13 15.7l0 160c0 8.8 7.2 16 16 16s16-7.2 16-16l0-76.9 33.1 82.9c2.8 7.1 10.3 11.2 17.9 9.8s13-8 13-15.7l0-160c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 76.9L95.8 170.6zm97.1 5.9l0 80 0 80c0 8.8 7.2 16 16 16s16-7.2 16-16l0-64 32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0 0-48 32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-48 0c-8.8 0-16 7.2-16 16zm112-16c-8.8 0-16 7.2-16 16s7.2 16 16 16l16 0 0 144c0 8.8 7.2 16 16 16s16-7.2 16-16l0-144 16 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-64 0z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faHexagonVerticalNft = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faHexagonVerticalNft.js
var import_faHexagonVerticalNft = __toESM(require_faHexagonVerticalNft());
var export_faHexagonVerticalNft = import_faHexagonVerticalNft.definition;
export {
  export_faHexagonVerticalNft as faHexagonVerticalNft
};
