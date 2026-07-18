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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMustache.js
var require_faMustache = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMustache.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mustache";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e5bc";
    var svgPathData = "M147.9 148.4C169.3 125.2 199.4 112 231 112c33 0 64.4 14.4 85.9 39.5l3.1 3.6 3.1-3.6C344.6 126.4 376 112 409 112c31.6 0 61.7 13.2 83.1 36.4l2 2.1c27.6 29.9 74.6 30.9 103.4 2.1L599 151c2.5-2.5 5.5-4.4 8.6-5.5c2.6-1 5.4-1.5 8.4-1.5c5.7 0 10.9 2 15 5.2c1.3 1.1 2.6 2.3 3.6 3.6c.4 .5 .7 1 1.1 1.4c.3 .4 .5 .8 .7 1.1c.3 .5 .6 .9 .8 1.4c1.8 3.3 2.7 7.1 2.7 11.2l0 39.4C640 313.8 553.8 400 447.4 400c-46.7 0-91.7-16.9-126.9-47.7l-.5-.4-.5 .4C284.4 383.1 239.3 400 192.6 400C86.2 400 0 313.8 0 207.4L0 168c0-4.2 1.1-8.1 2.9-11.5c.6-1 1.2-2.1 2-3c1.2-1.6 2.6-3 4.1-4.2c1.5-1.2 3.1-2.2 4.8-3c3.1-1.5 6.5-2.3 10.2-2.3c2.9 0 5.8 .5 8.4 1.5c3.1 1.2 6.1 3 8.6 5.5l1.6 1.6c28.8 28.8 75.8 27.9 103.4-2.1l2-2.1z";
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
    exports.faMustache = exports.definition;
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

// src/_entries/faMustache.js
var import_faMustache = __toESM(require_faMustache());
var export_faMustache = import_faMustache.definition;
export {
  export_faMustache as faMustache
};
