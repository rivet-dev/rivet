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

// src/node_modules/@fortawesome/free-solid-svg-icons/faDolly.js
var require_faDolly = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faDolly.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dolly";
    var width = 576;
    var height = 512;
    var aliases = ["dolly-box"];
    var unicode = "f472";
    var svgPathData = "M32 0C14.3 0 0 14.3 0 32S14.3 64 32 64l72.9 0 92.1 276.2c-22.5 17.6-37 45-37 75.8 0 53 43 96 96 96 52.4 0 95.1-42 96-94.3l202.1-67.4c16.8-5.6 25.8-23.7 20.2-40.5s-23.7-25.8-40.5-20.2L331.8 357c-17.2-22.1-43.9-36.5-74-37L165.7 43.8C156.9 17.6 132.5 0 104.9 0L32 0zM208 416a48 48 0 1 1 96 0 48 48 0 1 1 -96 0zM280.5 89.3c-25.2 8.2-39 35.3-30.8 60.5l39.6 121.7c8.2 25.2 35.3 39 60.5 30.8l121.7-39.6c25.2-8.2 39-35.3 30.8-60.5L462.8 80.5c-8.2-25.2-35.3-39-60.5-30.8L280.5 89.3z";
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
    exports.faDolly = exports.definition;
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

// src/_entries/faDollyBox.js
var import_faDolly = __toESM(require_faDolly());
var export_faDollyBox = import_faDolly.definition;
export {
  export_faDollyBox as faDollyBox
};
