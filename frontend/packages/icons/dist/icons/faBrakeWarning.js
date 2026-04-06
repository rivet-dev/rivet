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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBrakeWarning.js
var require_faBrakeWarning = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBrakeWarning.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "brake-warning";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e0c7";
    var svgPathData = "M96 256a224 224 0 1 1 448 0A224 224 0 1 1 96 256zM344 152c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 112c0 13.3 10.7 24 24 24s24-10.7 24-24l0-112zM320 384a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM91.1 109C63.8 151.4 48 201.8 48 256s15.8 104.6 43.1 147c7.2 11.1 4 26-7.2 33.2s-26 4-33.2-7.2C18.6 379.1 0 319.7 0 256S18.6 132.9 50.7 83c7.2-11.1 22-14.4 33.2-7.2s14.4 22 7.2 33.2zM589.3 83C621.4 132.9 640 192.3 640 256s-18.6 123.1-50.7 173c-7.2 11.1-22 14.4-33.2 7.2s-14.4-22-7.2-33.2c27.3-42.4 43.1-92.8 43.1-147s-15.8-104.6-43.1-147c-7.2-11.1-4-26 7.2-33.2s26-4 33.2 7.2z";
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
    exports.faBrakeWarning = exports.definition;
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

// src/_entries/faBrakeWarning.js
var import_faBrakeWarning = __toESM(require_faBrakeWarning());
var export_faBrakeWarning = import_faBrakeWarning.definition;
export {
  export_faBrakeWarning as faBrakeWarning
};
