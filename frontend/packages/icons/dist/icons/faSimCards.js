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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSimCards.js
var require_faSimCards = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSimCards.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sim-cards";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e251";
    var svgPathData = "M160 0C124.7 0 96 28.7 96 64l0 288c0 35.3 28.7 64 64 64l224 0c35.3 0 64-28.7 64-64l0-210.7c0-17-6.7-33.3-18.7-45.3L352 18.7C340 6.7 323.7 0 306.7 0L160 0zm48 144l0 48-48 0 0-16c0-17.7 14.3-32 32-32l16 0zm-48 80l64 0 96 0 64 0 0 48-64 0-96 0-64 0 0-48zm224-32l-48 0 0-48 16 0c17.7 0 32 14.3 32 32l0 16zm-80-48l0 48-64 0 0-48 64 0zm80 160l0 16c0 17.7-14.3 32-32 32l-16 0 0-48 48 0zm-80 48l-64 0 0-48 64 0 0 48zm-96 0l-16 0c-17.7 0-32-14.3-32-32l0-16 48 0 0 48zM48 120c0-13.3-10.7-24-24-24S0 106.7 0 120L0 376c0 75.1 60.9 136 136 136l224 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-224 0c-48.6 0-88-39.4-88-88l0-256z";
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
    exports.faSimCards = exports.definition;
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

// src/_entries/faSimCards.js
var import_faSimCards = __toESM(require_faSimCards());
var export_faSimCards = import_faSimCards.definition;
export {
  export_faSimCards as faSimCards
};
