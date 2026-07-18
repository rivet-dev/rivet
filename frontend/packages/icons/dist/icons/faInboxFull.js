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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faInboxFull.js
var require_faInboxFull = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faInboxFull.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "inbox-full";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e1ba";
    var svgPathData = "M58.9 80.5C66 52 91.6 32 121 32l270 0c29.4 0 55 20 62.1 48.5l57 227.9c1.3 5.1 1.9 10.3 1.9 15.5l0 92.1c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64l0-92.1c0-5.2 .6-10.4 1.9-15.5l57-227.9zM391 96L121 96 73 288l51.2 0c12.1 0 23.2 6.8 28.6 17.7l14.3 28.6c5.4 10.8 16.5 17.7 28.6 17.7l120.4 0c12.1 0 23.2-6.8 28.6-17.7l14.3-28.6c5.4-10.8 16.5-17.7 28.6-17.7l51.2 0L391 96zM176 144l160 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-160 0c-8.8 0-16-7.2-16-16s7.2-16 16-16zm-32 80l224 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-224 0c-8.8 0-16-7.2-16-16s7.2-16 16-16z";
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
    exports.faInboxFull = exports.definition;
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

// src/_entries/faInboxFull.js
var import_faInboxFull = __toESM(require_faInboxFull());
var export_faInboxFull = import_faInboxFull.definition;
export {
  export_faInboxFull as faInboxFull
};
