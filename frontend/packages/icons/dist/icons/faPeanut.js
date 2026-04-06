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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPeanut.js
var require_faPeanut = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPeanut.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "peanut";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e430";
    var svgPathData = "M432 0L364 0c-20.1 0-39.2 4-56.7 11.2c-17.4 7.2-33.7 17.8-47.8 32l-.4 .4C245 57.8 234.4 74 227.2 91.4c-4 9.5-6.9 19.6-8.8 30c-4.8 24.5-12.6 49.1-30.3 66.7s-42.2 25.4-66.7 30.3c-10.4 1.9-20.5 4.9-30 8.8c-17.5 7.2-33.9 17.9-48.1 32.1c-14.2 14.2-24.9 30.5-32.1 48C4 324.8 0 343.9 0 364l0 68c0 44.2 35.8 80 80 80l68 0c40.5 0 77.1-16.2 103.8-42.5c.6-.6 1.1-1.1 1.7-1.7c13.8-14 24.2-30.1 31.3-47.2c4-9.5 6.9-19.6 8.8-30c4.8-24.5 12.6-49.1 30.3-66.7s42.2-25.4 66.7-30.3c10.4-1.9 20.5-4.9 30-8.8c17.3-7.2 33.5-17.7 47.6-31.7l.9-.9C495.6 225.4 512 188.6 512 148l0-68c0-44.2-35.8-80-80-80zM396.8 128a19.2 19.2 0 1 1 38.4 0 19.2 19.2 0 1 1 -38.4 0zm0 64a19.2 19.2 0 1 1 38.4 0 19.2 19.2 0 1 1 -38.4 0zm-64 0a19.2 19.2 0 1 1 38.4 0 19.2 19.2 0 1 1 -38.4 0zm-224 224a19.2 19.2 0 1 1 38.4 0 19.2 19.2 0 1 1 -38.4 0zM192 396.8a19.2 19.2 0 1 1 0 38.4 19.2 19.2 0 1 1 0-38.4zm0-64a19.2 19.2 0 1 1 0 38.4 19.2 19.2 0 1 1 0-38.4z";
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
    exports.faPeanut = exports.definition;
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

// src/_entries/faPeanut.js
var import_faPeanut = __toESM(require_faPeanut());
var export_faPeanut = import_faPeanut.definition;
export {
  export_faPeanut as faPeanut
};
