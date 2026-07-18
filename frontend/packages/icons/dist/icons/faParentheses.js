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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBracketsRound.js
var require_faBracketsRound = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBracketsRound.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "brackets-round";
    var width = 512;
    var height = 512;
    var aliases = ["parentheses"];
    var unicode = "e0c5";
    var svgPathData = "M145.8 90.6c14.7-9.8 18.7-29.7 8.9-44.4s-29.7-18.7-44.4-8.9C71.9 62.9 0 140.2 0 256S71.9 449 110.2 474.6c14.7 9.8 34.6 5.8 44.4-8.9s5.8-34.6-8.9-44.4C120.1 404.3 64 345 64 256s56.1-148.3 81.8-165.4zm220.5 0C391.9 107.7 448 167 448 256s-56.1 148.3-81.8 165.4c-14.7 9.8-18.7 29.7-8.9 44.4s29.7 18.7 44.4 8.9C440.1 449.1 512 371.8 512 256s-71.9-193.1-110.2-218.6c-14.7-9.8-34.6-5.8-44.4 8.9s-5.8 34.6 8.9 44.4z";
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
    exports.faBracketsRound = exports.definition;
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

// src/_entries/faParentheses.js
var import_faBracketsRound = __toESM(require_faBracketsRound());
var export_faParentheses = import_faBracketsRound.definition;
export {
  export_faParentheses as faParentheses
};
