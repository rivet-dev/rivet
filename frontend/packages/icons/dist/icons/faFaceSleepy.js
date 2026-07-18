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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSleepy.js
var require_faFaceSleepy = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSleepy.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-sleepy";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e38e";
    var svgPathData = "M385.1 477.1C347.2 499.3 303.1 512 256 512C114.6 512 0 397.4 0 256S114.6 0 256 0S512 114.6 512 256c0 37-7.8 72.1-22 103.9c-7.4-6.2-15.1-11.8-22.7-16.7C429.6 318.7 383.6 304 352 304c-15.3 0-28.5 10.9-31.4 25.9s5.3 30.1 19.5 35.8c21 8.4 31.6 23.5 37.4 41.7c6.3 19.4 6.5 40.3 6.5 56.6c0 4.5 .4 8.9 1.1 13.1zM176 389.5c-6.8 11.1 6 20.7 18.3 16.4c18.9-6.5 39.8-10.1 61.8-10.1s42.9 3.6 61.8 10.1c12.3 4.2 25.1-5.3 18.3-16.4C321 364.8 290.8 348 256 348s-64.9 16.8-80.1 41.5zM316.8 246.4c-5.3-7.1-15.3-8.5-22.4-3.2s-8.5 15.3-3.2 22.4c30.4 40.5 91.2 40.5 121.6 0c5.3-7.1 3.9-17.1-3.2-22.4s-17.1-3.9-22.4 3.2c-17.6 23.5-52.8 23.5-70.4 0zm-121.6 0c-17.6 23.5-52.8 23.5-70.4 0c-5.3-7.1-15.3-8.5-22.4-3.2s-8.5 15.3-3.2 22.4c30.4 40.5 91.2 40.5 121.6 0c5.3-7.1 3.9-17.1-3.2-22.4s-17.1-3.9-22.4 3.2zM512 464c0 26.5-21.5 48-48 48s-48-21.5-48-48c0-32 0-102.4-64-128c48 0 160 48 160 128z";
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
    exports.faFaceSleepy = exports.definition;
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

// src/_entries/faFaceSleepy.js
var import_faFaceSleepy = __toESM(require_faFaceSleepy());
var export_faFaceSleepy = import_faFaceSleepy.definition;
export {
  export_faFaceSleepy as faFaceSleepy
};
