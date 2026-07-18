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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHexagonVerticalNftSlanted.js
var require_faHexagonVerticalNftSlanted = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHexagonVerticalNftSlanted.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hexagon-vertical-nft-slanted";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e506";
    var svgPathData = "M260.9 495.5c-22.3 12.9-49.7 12.9-72 0L36 407.2C13.7 394.4 0 370.6 0 344.9L0 168.3c0-25.7 13.7-49.5 36-62.4L188.9 17.6c22.3-12.9 49.7-12.9 72 0l152.9 88.3c22.3 12.9 36 36.6 36 62.4l0 176.6c0 25.7-13.7 49.5-36 62.4L260.9 495.5zm11.1-372c0-5.7-3.1-11-8.1-13.9L216.1 82.3c-2-1.2-4.3-2-6.8-2.2c-.5 0-1-.1-1.4-.1c0 0-.1 0-.1 0c-3.3 0-6.3 1-8.8 2.6c-1.5 1-2.9 2.2-4 3.7c-1.2 1.5-2.1 3.3-2.6 5.2c-.4 1.6-.6 3.1-.6 4.7l0 160.2 0 144c0 8.8 7.2 16 16 16s16-7.2 16-16l0-128 32 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-32 0 0-117 24.2 13.8c10.7 6.1 23.9-1.6 23.9-13.9zm16 27.4c0 5.7 3.1 11 8.1 13.9l23.8 13.6 0 174.2c0 8.8 7.2 16 16 16s16-7.2 16-16l0-155.9 16.2 9.3c7.7 4.4 17.4 1.7 21.8-6s1.7-17.4-6-21.8l-72-41.1c-10.7-6.1-23.9 1.6-23.9 13.9zM143.8 128.6c-8.8 0-16 7.2-16 16l0 125.4L95 171.5c-2.5-7.4-10-12-17.7-10.7s-13.4 7.9-13.4 15.8l0 160c0 8.8 7.2 16 16 16s16-7.2 16-16l0-61.4 32.8 98.5c2.5 7.4 10 12 17.7 10.7s13.4-7.9 13.4-15.8l0-224c0-8.8-7.2-16-16-16z";
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
    exports.faHexagonVerticalNftSlanted = exports.definition;
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

// src/_entries/faHexagonVerticalNftSlanted.js
var import_faHexagonVerticalNftSlanted = __toESM(require_faHexagonVerticalNftSlanted());
var export_faHexagonVerticalNftSlanted = import_faHexagonVerticalNftSlanted.definition;
export {
  export_faHexagonVerticalNftSlanted as faHexagonVerticalNftSlanted
};
