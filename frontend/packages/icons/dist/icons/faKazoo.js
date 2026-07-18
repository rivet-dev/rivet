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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faKazoo.js
var require_faKazoo = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faKazoo.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "kazoo";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8c7";
    var svgPathData = "M514 128l94 0c17.7 0 32 14.3 32 32l0 192c0 17.7-14.3 32-32 32l-94 0c-19.8 10.2-42.2 16-66 16s-46.3-5.8-66-16l-158 0c-3 0-6-.4-9-1.3l-192-56C9.4 322.7 0 310.2 0 296l0-80c0-14.2 9.4-26.7 23-30.7l192-56c2.9-.8 5.9-1.3 9-1.3l158 0c19.8-10.2 42.2-16 66-16s46.3 5.8 66 16zm30 128a96 96 0 1 0 -192 0 96 96 0 1 0 192 0zM427.3 212.7L448 233.4l20.7-20.7c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6L470.6 256l20.7 20.7c6.2 6.2 6.2 16.4 0 22.6s-16.4 6.2-22.6 0L448 278.6l-20.7 20.7c-6.2 6.2-16.4 6.2-22.6 0s-6.2-16.4 0-22.6L425.4 256l-20.7-20.7c-6.2-6.2-6.2-16.4 0-22.6s16.4-6.2 22.6 0z";
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
    exports.faKazoo = exports.definition;
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

// src/_entries/faKazoo.js
var import_faKazoo = __toESM(require_faKazoo());
var export_faKazoo = import_faKazoo.definition;
export {
  export_faKazoo as faKazoo
};
