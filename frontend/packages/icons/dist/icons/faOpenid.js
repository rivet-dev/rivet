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

// src/node_modules/@fortawesome/free-brands-svg-icons/faOpenid.js
var require_faOpenid = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faOpenid.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "openid";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f19b";
    var svgPathData = "M271.5 432l-68 32C88.5 453.7 0 392.5 0 318.2 0 246.7 82.5 187.2 191.7 173.9l0 43c-71.5 12.5-124 53-124 101.3 0 51 58.5 93.3 135.7 103l0-340 68-33.2 0 384 .1 0zM448 291l-131.3-28.5 36.8-20.7c-19.5-11.5-43.5-20-70-24.8l0-43c46.2 5.5 87.7 19.5 120.3 39.3l35-19.8 9.2 97.5z";
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
    exports.faOpenid = exports.definition;
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

// src/_entries/faOpenid.js
var import_faOpenid = __toESM(require_faOpenid());
var export_faOpenid = import_faOpenid.definition;
export {
  export_faOpenid as faOpenid
};
