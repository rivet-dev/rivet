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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMaskSnorkel.js
var require_faMaskSnorkel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMaskSnorkel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mask-snorkel";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e3b7";
    var svgPathData = "M544 32c-17.7 0-32 14.3-32 32l0 96 64 0 0-96c0-17.7-14.3-32-32-32zM512 367.4c0 44.5-36.1 80.6-80.6 80.6c-39.4 0-73-28.5-79.5-67.4l-.3-1.9c-2.9-17.4-19.4-29.2-36.8-26.3s-29.2 19.4-26.3 36.8l.3 1.9C300.4 460.9 360.7 512 431.4 512C511.3 512 576 447.3 576 367.4L576 192l-64 0 0 175.4zM64 64C28.7 64 0 92.7 0 128l0 96c0 35.3 28.7 64 64 64l79.7 0c28.6 0 53.7-18.9 61.5-46.4l3.4-11.7c4.4-15.5 26.3-15.5 30.8 0l3.4 11.7c7.8 27.5 33 46.4 61.5 46.4l79.7 0c35.3 0 64-28.7 64-64l0-96c0-35.3-28.7-64-64-64L64 64z";
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
    exports.faMaskSnorkel = exports.definition;
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

// src/_entries/faMaskSnorkel.js
var import_faMaskSnorkel = __toESM(require_faMaskSnorkel());
var export_faMaskSnorkel = import_faMaskSnorkel.definition;
export {
  export_faMaskSnorkel as faMaskSnorkel
};
