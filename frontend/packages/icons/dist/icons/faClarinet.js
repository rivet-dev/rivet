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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faClarinet.js
var require_faClarinet = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faClarinet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "clarinet";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8ad";
    var svgPathData = "M224 144c0-8.8 7.2-16 16-16l32 0 96 0 96 0 32 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-16 0 0 32 76.6 0c9.5 0 18.7-2.8 26.6-8.1l31.9-21.3c4.9-3.3 11.2-3.6 16.4-.8s8.5 8.2 8.5 14.1l0 192c0 5.9-3.2 11.3-8.5 14.1s-11.5 2.5-16.4-.8l-31.9-21.3c-7.9-5.3-17.1-8.1-26.6-8.1l-448.3 0c-10.2 0-20.3-1.9-29.7-5.7L10.1 318.9C4 316.4 0 310.5 0 304l0-64c0-6.5 4-12.4 10.1-14.9l68.6-27.4c9.4-3.8 19.5-5.7 29.7-5.7L256 192l0-32-16 0c-8.8 0-16-7.2-16-16zm224 16l-64 0 0 32 64 0 0-32zm-96 0l-64 0 0 32 64 0 0-32zM272 296a24 24 0 1 0 0-48 24 24 0 1 0 0 48zm120-24a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm72 24a24 24 0 1 0 0-48 24 24 0 1 0 0 48z";
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
    exports.faClarinet = exports.definition;
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

// src/_entries/faClarinet.js
var import_faClarinet = __toESM(require_faClarinet());
var export_faClarinet = import_faClarinet.definition;
export {
  export_faClarinet as faClarinet
};
