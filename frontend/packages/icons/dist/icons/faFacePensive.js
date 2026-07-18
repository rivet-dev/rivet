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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFacePensive.js
var require_faFacePensive = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFacePensive.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-pensive";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e384";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM208 384l96 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-96 0c-8.8 0-16-7.2-16-16s7.2-16 16-16zM387.2 270.4c5.3-7.1 15.3-8.5 22.4-3.2s8.5 15.3 3.2 22.4c-30.4 40.5-91.2 40.5-121.6 0c-5.3-7.1-3.9-17.1 3.2-22.4s17.1-3.9 22.4 3.2c17.6 23.5 52.8 23.5 70.4 0zm-262.4 0c17.6 23.5 52.8 23.5 70.4 0c5.3-7.1 15.3-8.5 22.4-3.2s8.5 15.3 3.2 22.4c-30.4 40.5-91.2 40.5-121.6 0c-5.3-7.1-3.9-17.1 3.2-22.4s17.1-3.9 22.4 3.2zM65.7 215.2c-4-7.9-.7-17.5 7.2-21.5l22.9-11.4c14.5-7.2 27.2-17.6 37.4-30.2l14.4-18c5.5-6.9 15.6-8 22.5-2.5s8 15.6 2.5 22.5l-14.4 18c-13 16.3-29.4 29.5-48 38.8L87.2 222.3c-7.9 4-17.5 .7-21.5-7.2zm373.5-21.5c7.9 4 11.1 13.6 7.2 21.5s-13.6 11.1-21.5 7.2L402 210.9c-18.6-9.3-35-22.6-48.1-38.8l-14.4-18c-5.5-6.9-4.4-17 2.5-22.5s17-4.4 22.5 2.5l14.4 18c10.1 12.7 22.9 23 37.4 30.2l22.9 11.4z";
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
    exports.faFacePensive = exports.definition;
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

// src/_entries/faFacePensive.js
var import_faFacePensive = __toESM(require_faFacePensive());
var export_faFacePensive = import_faFacePensive.definition;
export {
  export_faFacePensive as faFacePensive
};
