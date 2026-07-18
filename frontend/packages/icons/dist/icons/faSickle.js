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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSickle.js
var require_faSickle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSickle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sickle";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f822";
    var svgPathData = "M212.7 340.7L181 309c-33.9-33.9-53-80-53-128C128 81 209 0 309 0l11 0c90.1 0 165.8 62.1 186.4 145.8c1.8 7.3-4 14.2-11.5 14.2c-3.9 0-7.5-1.9-9.9-4.9C458.9 122.9 419.5 104 377.8 104C301.7 104 240 165.7 240 241.8l0 9.8c0 20.7 5.2 39.7 12.2 58.6c2.2 6 .9 12.8-3.7 17.3l-13.2 13.2c-6.2 6.2-16.4 6.2-22.6 0zM121 311l24 24 32 32 24 24c9.4 9.4 9.4 24.6 0 33.9l-16 16c-9.4 9.4-24.6 9.4-33.9 0l-7-7L73 505c-9.4 9.4-24.6 9.4-33.9 0L7 473c-9.4-9.4-9.4-24.6 0-33.9l71-71-7-7c-9.4-9.4-9.4-24.6 0-33.9l16-16c9.4-9.4 24.6-9.4 33.9 0z";
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
    exports.faSickle = exports.definition;
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

// src/_entries/faSickle.js
var import_faSickle = __toESM(require_faSickle());
var export_faSickle = import_faSickle.definition;
export {
  export_faSickle as faSickle
};
