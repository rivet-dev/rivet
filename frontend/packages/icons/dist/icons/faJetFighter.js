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

// src/node_modules/@fortawesome/free-solid-svg-icons/faJetFighter.js
var require_faJetFighter = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faJetFighter.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "jet-fighter";
    var width = 576;
    var height = 512;
    var aliases = ["fighter-jet"];
    var unicode = "f0fb";
    var svgPathData = "M496.2 206.8c-10.7-4.5-22.2-6.8-33.8-6.8L362 200 248 48 296 48c13.3 0 24-10.7 24-24S309.3 0 296 0L152 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l8 0 0 152-54.4 0-52.8-66c-3-3.8-7.6-6-12.5-6L16 128c-8.8 0-16 7.2-16 16l0 88 40 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-40 0 0 88c0 8.8 7.2 16 16 16l24.3 0c4.9 0 9.5-2.2 12.5-6l52.8-66 54.4 0 0 152-8 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l144 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-48 0 114-152 100.4 0c11.6 0 23.1-2.3 33.8-6.8l65-27.1c8.9-3.7 14.8-12.5 14.8-22.2s-5.8-18.4-14.8-22.2l-65-27.1z";
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
    exports.faJetFighter = exports.definition;
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

// src/_entries/faJetFighter.js
var import_faJetFighter = __toESM(require_faJetFighter());
var export_faJetFighter = import_faJetFighter.definition;
export {
  export_faJetFighter as faJetFighter
};
