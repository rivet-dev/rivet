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

// src/node_modules/@fortawesome/free-solid-svg-icons/fa6.js
var require_fa6 = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/fa6.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "6";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "36";
    var svgPathData = "M256 32c17.7 0 32 14.3 32 32s-14.3 32-32 32l-88 0c-48.6 0-88 39.4-88 88l0 32.2c22.9-15.3 50.4-24.2 80-24.2l48 0c79.5 0 144 64.5 144 144S287.5 480 208 480l-48 0C80.5 480 16 415.5 16 336l0-152C16 100.1 84.1 32 168 32l88 0zM80 336c0 44.2 35.8 80 80 80l48 0c44.2 0 80-35.8 80-80s-35.8-80-80-80l-48 0c-44.2 0-80 35.8-80 80z";
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
    exports.fa6 = exports.definition;
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

// src/_entries/fa6.js
var import_fa6 = __toESM(require_fa6());
var export_fa6 = import_fa6.definition;
export {
  export_fa6 as fa6
};
