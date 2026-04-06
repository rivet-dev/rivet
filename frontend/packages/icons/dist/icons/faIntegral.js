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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faIntegral.js
var require_faIntegral = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faIntegral.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "integral";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "f667";
    var svgPathData = "M216 64c-13.3 0-24 10.7-24 24l0 336c0 48.6-39.4 88-88 88s-88-39.4-88-88l0-8c0-17.7 14.3-32 32-32s32 14.3 32 32l0 8c0 13.3 10.7 24 24 24s24-10.7 24-24l0-336c0-48.6 39.4-88 88-88s88 39.4 88 88l0 8c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-8c0-13.3-10.7-24-24-24z";
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
    exports.faIntegral = exports.definition;
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

// src/_entries/faIntegral.js
var import_faIntegral = __toESM(require_faIntegral());
var export_faIntegral = import_faIntegral.definition;
export {
  export_faIntegral as faIntegral
};
