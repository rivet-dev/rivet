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

// src/node_modules/@fortawesome/free-solid-svg-icons/faWalkieTalkie.js
var require_faWalkieTalkie = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faWalkieTalkie.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "walkie-talkie";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "f8ef";
    var svgPathData = "M88-32c13.3 0 24 10.7 24 24l0 72 48 0c0-17.7 14.3-32 32-32s32 14.3 32 32l32 0c0-17.7 14.3-32 32-32s32 14.3 32 32l16 0c26.5 0 48 21.5 48 48l0 160.9c0 9.9-2.3 19.7-6.8 28.6l-20.2 40.4c-3.3 6.7-5.1 14-5.1 21.5l0 84.7c0 35.3-28.7 64-64 64L96 512c-35.3 0-64-28.7-64-64l0-84.7c0-7.5-1.7-14.8-5.1-21.5L6.8 301.5C2.3 292.6 0 282.8 0 272.9L0 112C0 85.5 21.5 64 48 64l16 0 0-72c0-13.3 10.7-24 24-24zm32 176c-13.3 0-24 10.7-24 24s10.7 24 24 24l144 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-144 0zm0 96c-13.3 0-24 10.7-24 24s10.7 24 24 24l144 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-144 0z";
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
    exports.faWalkieTalkie = exports.definition;
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

// src/_entries/faWalkieTalkie.js
var import_faWalkieTalkie = __toESM(require_faWalkieTalkie());
var export_faWalkieTalkie = import_faWalkieTalkie.definition;
export {
  export_faWalkieTalkie as faWalkieTalkie
};
