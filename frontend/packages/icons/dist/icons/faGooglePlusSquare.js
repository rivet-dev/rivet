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

// src/node_modules/@fortawesome/free-brands-svg-icons/faSquareGooglePlus.js
var require_faSquareGooglePlus = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faSquareGooglePlus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "square-google-plus";
    var width = 448;
    var height = 512;
    var aliases = ["google-plus-square"];
    var unicode = "f0d4";
    var svgPathData = "M448 96c0-35.3-28.7-64-64-64L64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320zM64 256c0-55.3 44.7-100 100-100 27 0 49.5 9.8 67 26.2l-27.1 26.1c-7.4-7.1-20.3-15.4-39.8-15.4-34.1 0-61.9 28.2-61.9 63.2 0 34.9 27.8 63.2 61.9 63.2 39.6 0 54.4-28.5 56.8-43.1l-56.9 0 0-34.4 94.4 0c1 5 1.6 10.1 1.6 16.6 0 57.1-38.3 97.6-96 97.6-55.3 0-100-44.7-100-100zm291 18.2l0 29-29.2 0 0-29-29 0 0-29.2 29 0 0-29 29.2 0 0 29 29 0 0 29.2-29 0z";
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
    exports.faSquareGooglePlus = exports.definition;
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

// src/_entries/faGooglePlusSquare.js
var import_faSquareGooglePlus = __toESM(require_faSquareGooglePlus());
var export_faGooglePlusSquare = import_faSquareGooglePlus.definition;
export {
  export_faGooglePlusSquare as faGooglePlusSquare
};
