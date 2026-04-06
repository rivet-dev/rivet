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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceVomit.js
var require_faFaceVomit = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceVomit.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-vomit";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e3a0";
    var svgPathData = "M544 256c0 72.5-30.1 137.9-78.5 184.5l-16.7-5.6c-11.9-4-21.5-12.3-27.2-22.9c16.7-12.6 26.4-27.7 26.4-44c0-44.2-71.6-80-160-80s-160 35.8-160 80c0 16.3 9.7 31.4 26.4 44c-5.7 10.6-15.2 19-27.2 22.9l-16.7 5.6C62.1 393.9 32 328.5 32 256C32 114.6 146.6 0 288 0S544 114.6 544 256zM148 141.1c0 2.8 1 5.5 2.8 7.6l36 43.2-36 43.2c-1.8 2.1-2.8 4.8-2.8 7.6c0 9 9.6 14.7 17.5 10.5l89.9-47.9c10.7-5.7 10.7-21.1 0-26.8l-89.9-47.9c-7.9-4.2-17.5 1.5-17.5 10.5zm262.5-10.5l-89.9 47.9c-10.7 5.7-10.7 21.1 0 26.8l89.9 47.9c7.9 4.2 17.5-1.5 17.5-10.5c0-2.8-1-5.5-2.8-7.6l-36-43.2 36-43.2c1.8-2.1 2.8-4.8 2.8-7.6c0-9-9.6-14.7-17.5-10.5zM32 448a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm480 0a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM192 396.1l0-57.4c23.6-11.1 57.3-18.7 96-18.7s72.4 7.6 96 18.7l0 57.4c0 31 19.8 58.5 49.2 68.3l34.1 11.4c7.6 2.5 12.7 9.6 12.7 17.6c0 10.3-8.3 18.6-18.6 18.6l-346.8 0c-10.3 0-18.6-8.3-18.6-18.6c0-8 5.1-15.1 12.7-17.6l34.1-11.4c29.4-9.8 49.2-37.3 49.2-68.3zM304 384a16 16 0 1 0 0 32 16 16 0 1 0 0-32zm-48 48a16 16 0 1 0 -32 0 16 16 0 1 0 32 0z";
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
    exports.faFaceVomit = exports.definition;
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

// src/_entries/faFaceVomit.js
var import_faFaceVomit = __toESM(require_faFaceVomit());
var export_faFaceVomit = import_faFaceVomit.definition;
export {
  export_faFaceVomit as faFaceVomit
};
