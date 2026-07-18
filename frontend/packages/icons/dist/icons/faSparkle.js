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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSparkle.js
var require_faSparkle = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSparkle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sparkle";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e5d6";
    var svgPathData = "M245.8 45.9C241.9 37.4 233.4 32 224 32s-17.9 5.4-21.8 13.9L142.7 174.7 13.9 234.2C5.4 238.1 0 246.6 0 256s5.4 17.9 13.9 21.8l128.8 59.5 59.5 128.8c3.9 8.5 12.4 13.9 21.8 13.9s17.9-5.4 21.8-13.9l59.5-128.8 128.8-59.5c8.5-3.9 13.9-12.4 13.9-21.8s-5.4-17.9-13.9-21.8L305.3 174.7 245.8 45.9z";
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
    exports.faSparkle = exports.definition;
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

// src/_entries/faSparkle.js
var import_faSparkle = __toESM(require_faSparkle());
var export_faSparkle = import_faSparkle.definition;
export {
  export_faSparkle as faSparkle
};
