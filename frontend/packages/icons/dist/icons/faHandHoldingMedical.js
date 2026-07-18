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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHandHoldingMedical.js
var require_faHandHoldingMedical = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHandHoldingMedical.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-holding-medical";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e05c";
    var svgPathData = "M240 24c0-13.3 10.7-24 24-24l48 0c13.3 0 24 10.7 24 24l0 56 56 0c13.3 0 24 10.7 24 24l0 48c0 13.3-10.7 24-24 24l-56 0 0 56c0 13.3-10.7 24-24 24l-48 0c-13.3 0-24-10.7-24-24l0-56-56 0c-13.3 0-24-10.7-24-24l0-48c0-13.3 10.7-24 24-24l56 0 0-56zM66.7 384l42.5-42.5c24-24 56.6-37.5 90.5-37.5L352 304c17.7 0 32 14.3 32 32s-14.3 32-32 32l-72 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l112.6 0 119.7-88.2c17.8-13.1 42.8-9.3 55.9 8.5s9.3 42.8-8.5 55.9L433.1 485.5c-23.4 17.2-51.6 26.5-80.7 26.5L32 512c-17.7 0-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32l34.7 0z";
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
    exports.faHandHoldingMedical = exports.definition;
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

// src/_entries/faHandHoldingMedical.js
var import_faHandHoldingMedical = __toESM(require_faHandHoldingMedical());
var export_faHandHoldingMedical = import_faHandHoldingMedical.definition;
export {
  export_faHandHoldingMedical as faHandHoldingMedical
};
