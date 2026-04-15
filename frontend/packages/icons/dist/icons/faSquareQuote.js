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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareQuote.js
var require_faSquareQuote = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareQuote.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-quote";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e329";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zm64 128l48 0c17.7 0 32 14.3 32 32l0 24 0 24 0 39.3c0 35.2-25.4 65.2-60.2 71l-7.9 1.3c-13.1 2.2-25.4-6.7-27.6-19.7s6.7-25.4 19.7-27.6l7.9-1.3c11.6-1.9 20.1-11.9 20.1-23.7l0-7.3-32 0c-17.7 0-32-14.3-32-32l0-48c0-17.7 14.3-32 32-32zm224 80l0 39.3c0 35.2-25.4 65.2-60.2 71l-7.9 1.3c-13.1 2.2-25.4-6.7-27.6-19.7s6.7-25.4 19.7-27.6l7.9-1.3c11.6-1.9 20.1-11.9 20.1-23.7l0-7.3-32 0c-17.7 0-32-14.3-32-32l0-48c0-17.7 14.3-32 32-32l48 0c17.7 0 32 14.3 32 32l0 24 0 24z";
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
    exports.faSquareQuote = exports.definition;
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

// src/_entries/faSquareQuote.js
var import_faSquareQuote = __toESM(require_faSquareQuote());
var export_faSquareQuote = import_faSquareQuote.definition;
export {
  export_faSquareQuote as faSquareQuote
};
