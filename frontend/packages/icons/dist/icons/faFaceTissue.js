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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceTissue.js
var require_faFaceTissue = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceTissue.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-tissue";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e39c";
    var svgPathData = "M512 256c0 141.4-114.6 256-256 256c-11.5 0-22.9-.8-34-2.2c.6-1.9 1-3.9 1.4-5.9l18.4-110.2 5.4 3.6c5.4 3.6 12.4 3.6 17.8 0L304 371.2l39.1 26.1c4.8 3.2 10.9 3.6 16 1l32-16c7.9-4 11.1-13.6 7.2-21.5s-13.6-11.1-21.5-7.2l-23.7 11.8-40.3-26.8c-5.4-3.6-12.4-3.6-17.8 0L256 364.8l-8.5-5.7 7.9-47.2c2.5-15.3-2.4-30.9-13.4-41.8s-26.5-16-41.8-13.4l-192 32c-2 .3-4 .8-5.9 1.4C.8 278.9 0 267.5 0 256C0 114.6 114.6 0 256 0S512 114.6 512 256zM133.5 130.7c-7.9-4.2-17.5 1.5-17.5 10.5c0 2.8 1 5.5 2.8 7.6l36 43.2-36 43.2c-1.8 2.1-2.8 4.8-2.8 7.6c0 9 9.6 14.7 17.5 10.5l89.9-47.9c10.7-5.7 10.7-21.1 0-26.8l-89.9-47.9zM396 141.1c0-9-9.6-14.7-17.5-10.5l-89.9 47.9c-10.7 5.7-10.7 21.1 0 26.8l89.9 47.9c7.9 4.2 17.5-1.5 17.5-10.5c0-2.8-1-5.5-2.8-7.6l-36-43.2 36-43.2c1.8-2.1 2.8-4.8 2.8-7.6zM219.3 292.7c3.7 3.7 5.3 8.8 4.5 13.9l-32 192c-1.1 6.7-6.3 11.9-13 13.1s-13.4-1.9-16.7-7.8l-26-45.5L51.9 479.5c-5.5 1.4-11.2-.2-15.2-4.2s-5.6-9.7-4.2-15.2l21.1-84.2-45.5-26c-5.9-3.4-9-10-7.8-16.7s6.4-11.9 13.1-13l192-32c5.1-.8 10.3 .8 13.9 4.5z";
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
    exports.faFaceTissue = exports.definition;
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

// src/_entries/faFaceTissue.js
var import_faFaceTissue = __toESM(require_faFaceTissue());
var export_faFaceTissue = import_faFaceTissue.definition;
export {
  export_faFaceTissue as faFaceTissue
};
