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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSquarePiedPiper.js
var require_faSquarePiedPiper = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSquarePiedPiper.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "square-pied-piper";
    var width = 448;
    var height = 512;
    var aliases = ["pied-piper-square"];
    var unicode = "e01e";
    var svgPathData = "M32.1 419l-32 60.2 .8-328C.9 85.3 54.1 32 120.1 32l327.2 0C354.3 60.9 257.4 126.2 193.4 200.6 122.8 282 82.7 338 32.1 419zm416-387S305.3 98.8 261.7 199.1c-23.2 53.6-28.9 118.1-71 158.6-28.9 27.8-69.8 38.2-105.3 56.3-23.2 12-66.4 40.5-84.9 66l328.4 0c66 0 119.3-53.3 119.3-119.2-.1 0-.1-328.8-.1-328.8z";
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
    exports.faSquarePiedPiper = exports.definition;
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

// src/_entries/faPiedPiperSquare.js
var import_faSquarePiedPiper = __toESM(require_faSquarePiedPiper());
var export_faPiedPiperSquare = import_faSquarePiedPiper.definition;
export {
  export_faPiedPiperSquare as faPiedPiperSquare
};
