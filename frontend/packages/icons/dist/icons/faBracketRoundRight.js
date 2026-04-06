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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBracketRoundRight.js
var require_faBracketRoundRight = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBracketRoundRight.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bracket-round-right";
    var width = 192;
    var height = 512;
    var aliases = [];
    var unicode = "29";
    var svgPathData = "M37.4 46.2C27.6 61 31.5 80.8 46.3 90.6C71.9 107.7 128 167 128 256s-56.1 148.3-81.8 165.4c-14.7 9.8-18.7 29.7-8.9 44.4s29.7 18.7 44.4 8.9C120.1 449 192 371.8 192 256S120.1 62.9 81.8 37.4C67 27.6 47.2 31.5 37.4 46.2z";
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
    exports.faBracketRoundRight = exports.definition;
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

// src/_entries/faBracketRoundRight.js
var import_faBracketRoundRight = __toESM(require_faBracketRoundRight());
var export_faBracketRoundRight = import_faBracketRoundRight.definition;
export {
  export_faBracketRoundRight as faBracketRoundRight
};
