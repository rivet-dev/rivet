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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRampLoading.js
var require_faRampLoading = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRampLoading.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ramp-loading";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f4d4";
    var svgPathData = "M32 48C32 21.5 53.5 0 80 0L368 0c26.5 0 48 21.5 48 48l0 392.4-64-128L352 64 96 64l0 248.4-64 128L32 48zM404.4 488.8c.8 1.5 1.2 3 1.5 4.6c1.5 9.3-5.6 18.6-15.8 18.6L368 512 80 512l-22.1 0c-10.2 0-17.4-9.2-15.8-18.6c.3-1.5 .7-3.1 1.5-4.6L96 384l32-64 192 0 32 64 52.4 104.8z";
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
    exports.faRampLoading = exports.definition;
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

// src/_entries/faRampLoading.js
var import_faRampLoading = __toESM(require_faRampLoading());
var export_faRampLoading = import_faRampLoading.definition;
export {
  export_faRampLoading as faRampLoading
};
