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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareW.js
var require_faSquareW = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSquareW.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "square-w";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e285";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zm54.9 136.7L160 297.3l41.1-128.6c3.2-9.9 12.4-16.7 22.9-16.7s19.7 6.7 22.9 16.7L288 297.3l41.1-128.6c4-12.6 17.5-19.6 30.2-15.5s19.6 17.5 15.5 30.2l-64 200c-3.2 9.9-12.4 16.7-22.9 16.7s-19.7-6.7-22.9-16.7L224 254.7 182.9 383.3c-3.2 9.9-12.4 16.7-22.9 16.7s-19.7-6.7-22.9-16.7l-64-200c-4-12.6 2.9-26.1 15.5-30.2s26.1 2.9 30.2 15.5z";
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
    exports.faSquareW = exports.definition;
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

// src/_entries/faSquareW.js
var import_faSquareW = __toESM(require_faSquareW());
var export_faSquareW = import_faSquareW.definition;
export {
  export_faSquareW as faSquareW
};
