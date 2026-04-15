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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStrawberry.js
var require_faStrawberry = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStrawberry.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "strawberry";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e32b";
    var svgPathData = "M481.5 1.9c12.2 5.2 17.8 19.3 12.6 31.5c-13.4 31.2-36.9 59-56.3 78.7c32.8 1.8 57.4 18 68.2 26.6c4 3.2 6 8.1 6 13.2s-2 10-6 13.3C495 174.4 469.7 192 440 192c-40 0-55.3-15.3-80-40s-40-40-40-80c0-29.7 17.6-55 26.7-66c3.3-4 8.2-6 13.3-6s10 2 13.2 6C382.3 17.4 400 44.3 400 80c0 .7 0 1.4 0 2.1c.7-.7 1.4-1.3 2-2c18.2-18.2 37.7-41.7 47.9-65.5c5.2-12.2 19.3-17.8 31.5-12.6zM0 336C0 160 96.2 64 192.2 64c32.7 .1 65.3 12.6 90.3 37.5l128 128c25 25 37.5 57.8 37.5 90.5c0 96-96 192-272 192C48 512 0 464 0 336zM172.8 172.8c11.9-11.9 14.6-28.3 8.7-34.3s-22.4-3.2-34.3 8.7s-14.6 28.3-8.7 34.3s22.4 3.2 34.3-8.7zm72.7 29.7c-5.9-5.9-22.4-3.2-34.3 8.7s-14.6 28.3-8.7 34.3s22.4 3.2 34.3-8.7s14.6-28.3 8.7-34.3zm-32 96c-5.9-5.9-22.4-3.2-34.3 8.7s-14.6 28.3-8.7 34.3s22.4 3.2 34.3-8.7s14.6-28.3 8.7-34.3zm-32 96c-5.9-5.9-22.4-3.2-34.3 8.7s-14.6 28.3-8.7 34.3s22.4 3.2 34.3-8.7s14.6-28.3 8.7-34.3zm87.3 2.3c11.9-11.9 14.6-28.3 8.7-34.3s-22.4-3.2-34.3 8.7s-14.6 28.3-8.7 34.3s22.4 3.2 34.3-8.7zm32-96c11.9-11.9 14.6-28.3 8.7-34.3s-22.4-3.2-34.3 8.7s-14.6 28.3-8.7 34.3s22.4 3.2 34.3-8.7zm72.7 29.7c-5.9-5.9-22.4-3.2-34.3 8.7s-14.6 28.3-8.7 34.3s22.4 3.2 34.3-8.7s14.6-28.3 8.7-34.3zm-224-96c-5.9-5.9-22.4-3.2-34.3 8.7s-14.6 28.3-8.7 34.3s22.4 3.2 34.3-8.7s14.6-28.3 8.7-34.3zM108.8 364.8c11.9-11.9 14.6-28.3 8.7-34.3s-22.4-3.2-34.3 8.7s-14.6 28.3-8.7 34.3s22.4 3.2 34.3-8.7z";
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
    exports.faStrawberry = exports.definition;
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

// src/_entries/faStrawberry.js
var import_faStrawberry = __toESM(require_faStrawberry());
var export_faStrawberry = import_faStrawberry.definition;
export {
  export_faStrawberry as faStrawberry
};
