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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHandHoldingHand.js
var require_faHandHoldingHand = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHandHoldingHand.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-holding-hand";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e4f7";
    var svgPathData = "M466.8 186.5l42.5-42.5 34.7 0c17.7 0 32-14.3 32-32l0-64c0-17.7-14.3-32-32-32L223.6 16c-29 0-57.3 9.3-80.7 26.5L16.3 135.8c-17.8 13.1-21.6 38.1-8.5 55.9s38.1 21.6 55.9 8.5L183.4 112 296 112c13.3 0 24 10.7 24 24s-10.7 24-24 24l-72 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l152.2 0c33.9 0 66.5-13.5 90.5-37.5zm-357.5 139L66.7 368 32 368c-17.7 0-32 14.3-32 32l0 64c0 17.7 14.3 32 32 32l320.5 0c29 0 57.3-9.3 80.7-26.5l126.6-93.3c17.8-13.1 21.6-38.1 8.5-55.9s-38.1-21.6-55.9-8.5L392.6 400 280 400c-13.3 0-24-10.7-24-24s10.7-24 24-24l72 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-152.2 0c-33.9 0-66.5 13.5-90.5 37.5z";
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
    exports.faHandHoldingHand = exports.definition;
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

// src/_entries/faHandHoldingHand.js
var import_faHandHoldingHand = __toESM(require_faHandHoldingHand());
var export_faHandHoldingHand = import_faHandHoldingHand.definition;
export {
  export_faHandHoldingHand as faHandHoldingHand
};
