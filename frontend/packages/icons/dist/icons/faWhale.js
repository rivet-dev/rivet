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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faWhale.js
var require_faWhale = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faWhale.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "whale";
    var width = 640;
    var height = 512;
    var aliases = [128011];
    var unicode = "f72c";
    var svgPathData = "M175.8 13.4c-1-5.9-5.2-10.8-10.9-12.6s-12-.3-16.2 3.9L108 45.4C89.7 63.7 81.4 89.6 85.6 115.1l7.9 47.4L43.6 212.4C15.7 240.3 0 278.2 0 317.6C0 407.3 72.7 480 162.4 480L544 480c53 0 96-43 96-96l0-111.9C640 192.5 575.5 128 495.9 128c-82.5 0-162.4 29.5-225 83.2L156.4 309.4c-8 6.8-18.1 10.6-28.6 10.6l-1.1 0c-18.3 0-34.5-11.7-40.3-29c-4-12-2.4-25.2 4.3-35.9l37.6-60.1 44.7 7.5c25.5 4.2 51.5-4.1 69.7-22.3l40.7-40.7c4.2-4.2 5.8-10.5 3.9-16.2s-6.7-9.9-12.6-10.9L189.9 98.1 175.8 13.4zM400 280a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faWhale = exports.definition;
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

// src/_entries/faWhale.js
var import_faWhale = __toESM(require_faWhale());
var export_faWhale = import_faWhale.definition;
export {
  export_faWhale as faWhale
};
