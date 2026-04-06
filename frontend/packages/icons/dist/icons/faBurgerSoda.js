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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBurgerSoda.js
var require_faBurgerSoda = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBurgerSoda.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "burger-soda";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f858";
    var svgPathData = "M249.3 0c-26 0-48.6 17.9-54.5 43.2l-20 84.8L32 128c-17.7 0-32 14.3-32 32s14.3 32 32 32l301.4 0c14-8.2 30.1-15.4 48.6-20.9c1.3-3.5 2-7.2 2-11.1c0-17.7-14.3-32-32-32l-127.8 0 17.4-73.8c.9-3.6 4.1-6.2 7.8-6.2L280 48c13.3 0 24-10.7 24-24s-10.7-24-24-24L249.3 0zM40 224L60.3 468c2.1 24.9 22.9 44 47.8 44L272 512c-10-13.4-16-30-16-48c0-14.7 6.6-27.8 17-36.6c-10.5-11.4-17-26.6-17-43.4c0-18.5 7.8-35.1 20.3-46.8c-12.4-10.9-20.3-26.9-20.3-44.8c0-6.3 1-13.8 4.4-21.5c4.2-9.5 13.8-27.8 32.2-47L40 224zm600 68.3c0-3-.5-5.9-1.7-8.6c-8.1-18.4-48.4-91.9-174.3-91.9s-166.2 73.5-174.3 91.9c-1.2 2.7-1.7 5.7-1.7 8.6c0 15.2 12.3 27.5 27.5 27.5l296.9 0c15.2 0 27.5-12.3 27.5-27.5zM384 239.8a16 16 0 1 1 0 32 16 16 0 1 1 0-32zm64 0a16 16 0 1 1 32 0 16 16 0 1 1 -32 0zm96 0a16 16 0 1 1 0 32 16 16 0 1 1 0-32zm-224 112c-17.7 0-32 14.3-32 32s14.3 32 32 32l288 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-288 0zm-16 96c-8.8 0-16 7.2-16 16c0 26.5 21.5 48 48 48l256 0c26.5 0 48-21.5 48-48c0-8.8-7.2-16-16-16l-320 0z";
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
    exports.faBurgerSoda = exports.definition;
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

// src/_entries/faBurgerSoda.js
var import_faBurgerSoda = __toESM(require_faBurgerSoda());
var export_faBurgerSoda = import_faBurgerSoda.definition;
export {
  export_faBurgerSoda as faBurgerSoda
};
