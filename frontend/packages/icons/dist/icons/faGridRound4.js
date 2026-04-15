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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGridRound4.js
var require_faGridRound4 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGridRound4.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "grid-round-4";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e5dd";
    var svgPathData = "M44.8 121.6a44.8 44.8 0 1 1 0-89.6 44.8 44.8 0 1 1 0 89.6zm0 119.5a44.8 44.8 0 1 1 0-89.6 44.8 44.8 0 1 1 0 89.6zm44.8 74.7A44.8 44.8 0 1 1 0 315.7a44.8 44.8 0 1 1 89.6 0zM44.8 480a44.8 44.8 0 1 1 0-89.6 44.8 44.8 0 1 1 0 89.6zM209.1 76.8a44.8 44.8 0 1 1 -89.6 0 44.8 44.8 0 1 1 89.6 0zM164.3 241.1a44.8 44.8 0 1 1 0-89.6 44.8 44.8 0 1 1 0 89.6zm44.8 74.7a44.8 44.8 0 1 1 -89.6 0 44.8 44.8 0 1 1 89.6 0zM164.3 480a44.8 44.8 0 1 1 0-89.6 44.8 44.8 0 1 1 0 89.6zM328.5 76.8a44.8 44.8 0 1 1 -89.6 0 44.8 44.8 0 1 1 89.6 0zM283.7 241.1a44.8 44.8 0 1 1 0-89.6 44.8 44.8 0 1 1 0 89.6zm44.8 74.7a44.8 44.8 0 1 1 -89.6 0 44.8 44.8 0 1 1 89.6 0zM283.7 480a44.8 44.8 0 1 1 0-89.6 44.8 44.8 0 1 1 0 89.6zM448 76.8a44.8 44.8 0 1 1 -89.6 0 44.8 44.8 0 1 1 89.6 0zM403.2 241.1a44.8 44.8 0 1 1 0-89.6 44.8 44.8 0 1 1 0 89.6zM448 315.7a44.8 44.8 0 1 1 -89.6 0 44.8 44.8 0 1 1 89.6 0zM403.2 480a44.8 44.8 0 1 1 0-89.6 44.8 44.8 0 1 1 0 89.6z";
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
    exports.faGridRound4 = exports.definition;
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

// src/_entries/faGridRound4.js
var import_faGridRound4 = __toESM(require_faGridRound4());
var export_faGridRound4 = import_faGridRound4.definition;
export {
  export_faGridRound4 as faGridRound4
};
