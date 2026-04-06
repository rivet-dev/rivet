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

// src/node_modules/@fortawesome/free-brands-svg-icons/faDeskpro.js
var require_faDeskpro = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faDeskpro.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "deskpro";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f38f";
    var svgPathData = "M122 406.2l84 0c23.1 0 43.9-3.5 62.6-10.7 18.7-7.1 34.7-17.1 48-30.1 13.3-13.4 23.6-29.2 30.7-47.5 3.9-9.6 6.8-19.8 8.6-30.6l73.7 0c-2.5 21.3-7.7 41.3-15.5 60-11.1 27.2-26.9 50.8-47.3 70.9-20 19.6-44.2 34.8-72.6 45.4s-60 16-94.7 16l-149.8 .2 0-192.4 72.3-.2 0 118.8zM199.4 32c34.7 0 66.2 5.4 94.7 16 28.5 10.7 52.9 26.1 73.3 46.2 20.4 19.6 36.2 43.2 47.3 70.9 7.5 18.3 12.4 38.1 14.9 59.3l-73.7-.1c-1.9-10.5-4.7-20.5-8.6-29.9-7.1-18.7-17.3-34.5-30.7-47.5-13.3-13.4-29.3-23.6-48-30.8-18.6-7.1-39.5-10.7-62.7-10.7l-84 0 0 118.8-72.2 .2 0-192.3 149.7-.1z";
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
    exports.faDeskpro = exports.definition;
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

// src/_entries/faDeskpro.js
var import_faDeskpro = __toESM(require_faDeskpro());
var export_faDeskpro = import_faDeskpro.definition;
export {
  export_faDeskpro as faDeskpro
};
