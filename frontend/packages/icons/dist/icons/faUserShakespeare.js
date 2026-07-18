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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserShakespeare.js
var require_faUserShakespeare = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserShakespeare.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-shakespeare";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e2c2";
    var svgPathData = "M224 48c-44.2 0-80 35.8-80 80l0 32c0 40.1 29.5 73.3 68 79.1l0-15.1c0-6.6 5.4-12 12-12s12 5.4 12 12l0 15.1c38.5-5.8 68-39 68-79.1l0-32c0-44.2-35.8-80-80-80zM96 128C96 57.3 153.3 0 224 0s128 57.3 128 128l0 23.2c0 10.9 3.7 21.5 10.5 30L373.6 195c6.7 8.4 10.4 18.8 10.4 29.6c0 26.2-21.2 47.4-47.4 47.4L286 272c-18.4 10.2-39.5 16-62 16s-43.6-5.8-62-16l-50.6 0C85.2 272 64 250.8 64 224.6c0-10.8 3.7-21.2 10.4-29.6l11.1-13.9c6.8-8.5 10.5-19.1 10.5-30L96 128zm96-16a16 16 0 1 1 0 32 16 16 0 1 1 0-32zm48 16a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zm-7.5 39.5c2 2 6 5.3 10.7 8.1c4.9 2.9 9.4 4.4 12.8 4.4c6.6 0 12 5.4 12 12s-5.4 12-12 12c-9.5 0-18.3-3.8-24.8-7.6c-2.5-1.5-4.9-3.1-7.1-4.6C211.6 201 199 204 192 204c-6.6 0-12-5.4-12-12s5.4-12 12-12c2.1 0 12.9-1.9 23.5-12.5c4.7-4.7 12.3-4.7 17 0zM272 304c23.9 0 45.8 8.8 62.6 23.2c65.7 20.4 113.4 81.7 113.4 154c0 17-13.8 30.7-30.7 30.7L240 512c0-8.8-7.2-16-16-16s-16 7.2-16 16L30.7 512C13.8 512 0 498.2 0 481.3c0-72.4 47.7-133.6 113.4-154C130.2 312.8 152.1 304 176 304l96 0zM208 448a16 16 0 1 0 32 0 16 16 0 1 0 -32 0zm-80-48c26.4-10.6 54.6-16 83.1-16l25.8 0c28.5 0 56.7 5.4 83.1 16c0-26.5-21.5-48-48-48l-96 0c-26.5 0-48 21.5-48 48z";
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
    exports.faUserShakespeare = exports.definition;
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

// src/_entries/faUserShakespeare.js
var import_faUserShakespeare = __toESM(require_faUserShakespeare());
var export_faUserShakespeare = import_faUserShakespeare.definition;
export {
  export_faUserShakespeare as faUserShakespeare
};
