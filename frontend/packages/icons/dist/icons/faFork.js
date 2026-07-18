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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFork.js
var require_faFork = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFork.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fork";
    var width = 512;
    var height = 512;
    var aliases = ["utensil-fork"];
    var unicode = "f2e3";
    var svgPathData = "M412.4 5.9c5.2 6.4 4.8 15.6-1.1 21.5L320 118.6c-5.2 5.2-5.2 13.6 0 18.8c4.9 4.9 12.8 5.2 18.1 .6l99.4-86.1c6.3-5.5 15.9-5.2 21.8 .8s6.3 15.4 .8 21.8L374 173.9c-4.6 5.3-4.3 13.2 .6 18.1c5.2 5.2 13.6 5.2 18.8 0l91.3-91.3c5.8-5.8 15.1-6.3 21.5-1.1s7.7 14.4 3.2 21.2l-71 106.5c-18.6 27.9-49.9 44.6-83.4 44.6c-19.3 0-37.9-5.5-53.9-15.7L54.6 502.6c-12.5 12.5-32.8 12.5-45.3 0s-12.5-32.8 0-45.3L255.7 211c-10.2-16-15.7-34.7-15.7-53.9c0-33.5 16.8-64.8 44.6-83.4l106.5-71c6.8-4.6 16-3.2 21.2 3.2z";
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
    exports.faFork = exports.definition;
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

// src/_entries/faFork.js
var import_faFork = __toESM(require_faFork());
var export_faFork = import_faFork.definition;
export {
  export_faFork as faFork
};
