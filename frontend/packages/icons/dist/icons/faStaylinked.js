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

// src/node_modules/@fortawesome/free-brands-svg-icons/faStaylinked.js
var require_faStaylinked = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faStaylinked.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "staylinked";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f3f5";
    var svgPathData = "M386.7 292.5l2.7 2.7-170-167.3c-3.5-3.5-9.7-3.7-13.8-.5L148.3 171c-4.2 3.2-4.6 8.7-1.1 12.2l68.1 64.3c3.6 3.5 9.9 3.7 14 .5l.1-.1c4.1-3.2 10.4-3 14 .5l84 81.3c3.6 3.5 3.2 9-.9 12.2l-93.2 74c-4.2 3.3-10.5 3.1-14.2-.4L67.2 268c-3.5-3.5-9.7-3.7-13.9-.5L7.5 302.4c-4.2 3.2-4.7 8.7-1.2 12.2L215 510.7s7.4 6.8 17.3-.8L430.3 346c4-3.2 4.4-8.7 .7-12.2l-44.3-41.3zm54.5-83.4L230.7 2.5c-1.5-1.2-8-5.5-16.3 1.1L7.6 165.7c-4.2 3.2-4.8 8.7-1.2 12.2l42.3 41.7 171.7 165.1c3.7 3.5 10.1 3.7 14.3 .4l50.2-38.8-.3-.3 7.7-6c4.2-3.2 4.6-8.7 .9-12.2l-57.1-54.4c-3.6-3.5-10-3.7-14.2-.5l-.1 .1c-4.2 3.2-10.5 3.1-14.2-.4L113 180.8c-3.6-3.5-3.1-8.9 1.1-12.2l92.2-71.5c4.1-3.2 10.3-3 13.9 .5l160.4 159c3.7 3.5 10 3.7 14.1 .5l45.8-35.8c4.1-3.2 4.4-8.7 .7-12.2z";
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
    exports.faStaylinked = exports.definition;
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

// src/_entries/faStaylinked.js
var import_faStaylinked = __toESM(require_faStaylinked());
var export_faStaylinked = import_faStaylinked.definition;
export {
  export_faStaylinked as faStaylinked
};
